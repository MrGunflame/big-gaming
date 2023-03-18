use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use std::time::{Duration, Instant};

use futures::FutureExt;
use game_common::entity::EntityId;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

use crate::entity::Entities;
use crate::proto::ack::{Ack, Nak};
use crate::proto::handshake::{Handshake, HandshakeFlags, HandshakeType};
use crate::proto::sequence::Sequence;
use crate::proto::shutdown::{Shutdown, ShutdownReason};
use crate::proto::timestamp::Timestamp;
use crate::proto::{Encode, Frame, Header, Packet, PacketBody};
use crate::snapshot::{Command, CommandQueue, ConnectionMessage};
use crate::socket::Socket;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorKind);

#[derive(Debug, Error)]
enum ErrorKind {
    #[error("connection refused")]
    ConnectionRefused,
    #[error("timed out")]
    TimedOut,
    #[error("peer shutdown")]
    PeerShutdown,
}

static CONNECTION_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u32);

impl ConnectionId {
    #[inline]
    pub fn new() -> Self {
        let id = CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

impl Default for ConnectionId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct Connection {
    pub id: ConnectionId,
    /// Input stream from the socket
    stream: mpsc::Receiver<Packet>,
    socket: Arc<Socket>,

    /// Direction (from self)
    chan_out: mpsc::Receiver<Command>,

    state: ConnectionState,

    queue: CommandQueue,
    entities: Entities,
    peer: SocketAddr,
    backlog: Backlog,
    frame_queue: FrameQueue,
    mode: ConnectionMode,
    write: Option<WriteRequest>,
    interval: TickInterval,
    last_time: Instant,
    next_server_sequence: Sequence,
    start_time: Instant,

    next_peer_sequence: Sequence,
}

impl Connection {
    pub fn new(
        peer: SocketAddr,
        queue: CommandQueue,
        socket: Arc<Socket>,
        mode: ConnectionMode,
    ) -> (Self, ConnectionHandle) {
        let id = ConnectionId::new();

        let (tx, rx) = mpsc::channel(32);
        let (out_tx, out_rx) = mpsc::channel(32);

        let mut conn = Self {
            id,
            stream: rx,
            socket,
            state: ConnectionState::Handshake(HandshakeState::Hello),
            chan_out: out_rx,
            queue,
            entities: Entities::new(),
            peer,
            backlog: Backlog::new(),
            frame_queue: FrameQueue::new(),
            mode,
            write: None,
            interval: TickInterval::new(),
            last_time: Instant::now(),
            next_server_sequence: Sequence::default(),
            start_time: Instant::now(),
            next_peer_sequence: Sequence::default(),
        };

        if mode == ConnectionMode::Connect {
            conn.prepare_connect();
        }

        (
            conn,
            ConnectionHandle {
                id,
                tx,
                chan_out: out_tx,
            },
        )
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ErrorKind>> {
        tracing::trace!("Connection.poll_read");

        while let Poll::Ready(packet) = self.stream.poll_recv(cx) {
            tracing::debug!("recv {:?}", packet);

            self.last_time = Instant::now();

            let Some(packet) = packet else {
                return self.abort();
            };

            if self.handle_packet(packet).is_ready() {
                return Poll::Ready(Ok(()));
            }
        }

        if let Poll::Ready(res) = self.poll_tick(cx) {
            return Poll::Ready(res);
        }

        // Don't send commands to peer until connected.
        if self.state != ConnectionState::Connected {
            return Poll::Pending;
        }

        while let Poll::Ready(cmd) = self.chan_out.poll_recv(cx) {
            let Some(cmd) = cmd else {
                return self.shutdown();
            };

            let frame = match self.entities.pack(&cmd) {
                Some(frame) => frame,
                None => {
                    tracing::info!("backlogging command {:?} for entity {:?}", cmd, cmd.id());

                    if let Some(id) = cmd.id() {
                        self.backlog.insert(id, cmd);
                    }

                    continue;
                }
            };

            self.frame_queue.push(frame);

            if let Some(id) = cmd.id() {
                if let Some(vec) = self.backlog.remove(id) {
                    tracing::info!("flushing commands for {:?}", id);

                    self.frame_queue
                        .extend(vec.into_iter().map(|cmd| self.entities.pack(&cmd).unwrap()));
                }
            }
        }

        if let Some(frame) = self.frame_queue.pop() {
            self.send(PacketBody::Frames(vec![frame]), ConnectionState::Connected);
            return Poll::Ready(Ok(()));
        }

        Poll::Pending
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        tracing::trace!("Connection.poll_write");

        #[cfg(debug_assertions)]
        assert!(self.write.is_some());

        let Some(req) = &mut self.write else {
            unreachable!();
        };

        match req.future.poll_unpin(cx) {
            Poll::Ready(_) => {
                self.state = req.state;
                self.write = None;
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }

    ///
    /// Returns `Poll::Ready` on state change.
    fn handle_packet(&mut self, packet: Packet) -> Poll<()> {
        match packet.body {
            PacketBody::Handshake(body) => self.handle_handshake(packet.header, body),
            PacketBody::Shutdown(body) => self.handle_shutdown(packet.header, body),
            PacketBody::Ack(body) => self.handle_ack(packet.header, body),
            PacketBody::Nak(body) => self.handle_nak(packet.header, body),
            PacketBody::Frames(body) => self.handle_data(packet.header, body),
        }
    }

    fn handle_data(&mut self, header: Header, frames: Vec<Frame>) -> Poll<()> {
        self.next_peer_sequence += 1;

        let snapshot = self.start_time + header.timestamp.to_duration();

        for frame in frames {
            let Some(cmd) = self.entities.unpack(frame) else {
                    tracing::debug!("failed to translate cmd");
                    continue;
                };

            self.queue.push(ConnectionMessage {
                id: self.id,
                snapshot,
                command: cmd,
            });
        }

        Poll::Ready(())
    }

    fn handle_handshake(&mut self, header: Header, body: Handshake) -> Poll<()> {
        // Ignore if not in HS process.
        let ConnectionState::Handshake(state) = self.state else {
            return Poll::Ready(());
        };

        match (state, self.mode) {
            // Connect mode
            (HandshakeState::Hello, ConnectionMode::Connect) => {
                assert_eq!(body.kind, HandshakeType::HELLO);

                if body.kind != HandshakeType::HELLO {
                    tracing::info!("abort: expected HELLO, but got {:?}", body.kind);
                    self.abort();
                    return Poll::Ready(());
                }

                // Send AGREEMENT
                let resp = Handshake {
                    version: 0,
                    kind: HandshakeType::AGREEMENT,
                    flags: HandshakeFlags::default(),
                    mtu: 1500,
                    flow_window: 8192,
                };

                return self.send(resp, ConnectionState::Handshake(HandshakeState::Agreement));
            }
            (HandshakeState::Agreement, ConnectionMode::Connect) => {
                if body.kind != HandshakeType::AGREEMENT {
                    tracing::info!("abort: expected AGREEMENT, but got {:?}", body.kind);
                    self.abort();
                    return Poll::Ready(());
                }

                self.state = ConnectionState::Connected;
                self.queue.push(ConnectionMessage {
                    id: self.id,
                    snapshot: self.start_time,
                    command: Command::Connected,
                });
            }
            // Listen mode
            (HandshakeState::Hello, ConnectionMode::Listen) => {
                if body.kind != HandshakeType::HELLO {
                    tracing::info!("reject: expected HELLO, but got {:?}", body.kind);
                    return self.reject(HandshakeType::REJ_ROGUE);
                }

                // Send HELLO
                let resp = Handshake {
                    version: 0,
                    kind: HandshakeType::HELLO,
                    flags: HandshakeFlags::default(),
                    mtu: 1500,
                    flow_window: 8192,
                };

                return self.send(resp, ConnectionState::Handshake(HandshakeState::Agreement));
            }
            (HandshakeState::Agreement, ConnectionMode::Listen) => {
                if body.kind != HandshakeType::AGREEMENT {
                    tracing::info!("reject: expected AGREEMENT, but got {:?}", body.kind);
                    return self.reject(HandshakeType::REJ_ROGUE);
                }

                let resp = Handshake {
                    version: 0,
                    kind: HandshakeType::AGREEMENT,
                    flags: HandshakeFlags::default(),
                    mtu: 1500,
                    flow_window: 8192,
                };

                // Signal the game that the player spawns.
                self.queue.push(ConnectionMessage {
                    id: self.id,
                    snapshot: self.start_time,
                    command: Command::Connected,
                });

                return self.send(resp, ConnectionState::Connected);
            }
        }

        Poll::Pending
    }

    fn handle_shutdown(&mut self, header: Header, body: Shutdown) -> Poll<()> {
        let _ = self.shutdown();
        Poll::Ready(())
    }

    fn handle_ack(&mut self, header: Header, body: Ack) -> Poll<()> {
        Poll::Pending
    }

    fn handle_nak(&mut self, header: Header, body: Nak) -> Poll<()> {
        Poll::Pending
    }

    fn prepare_connect(&mut self) {
        let req = Handshake {
            version: 0,
            kind: HandshakeType::HELLO,
            flags: HandshakeFlags::default(),
            mtu: 1500,
            flow_window: 8192,
        };

        // connect is only called initially (before the future was first polled).
        let _ = self.send(req, ConnectionState::Handshake(HandshakeState::Hello));
    }

    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ErrorKind>> {
        ready!(self.interval.poll_tick(cx));

        if self.last_time.elapsed() >= Duration::from_secs(15) {
            tracing::info!("closing connection due to timeout");

            self.shutdown();
            return Poll::Ready(Err(ErrorKind::TimedOut));
        }

        // Send periodic ACKs while connected.
        if self.state == ConnectionState::Connected {
            let _ = self.send(
                Ack {
                    sequence: self.next_peer_sequence,
                },
                ConnectionState::Connected,
            );
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn reject(&mut self, reason: HandshakeType) -> Poll<()> {
        // Don't accidently send a non-rejection.
        #[cfg(debug_assertions)]
        assert!(reason.is_rejection());

        let resp = Handshake {
            version: 0,
            kind: reason,
            flags: HandshakeFlags::default(),
            mtu: 1500,
            flow_window: 8192,
        };

        self.send(resp, ConnectionState::Closed)
    }

    /// Closes the connection without doing a shutdown process.
    fn abort(&mut self) -> Poll<Result<(), ErrorKind>> {
        // If the connection active we need to notify that the player left.
        if self.state == ConnectionState::Connected {
            self.queue.push(ConnectionMessage {
                id: self.id,
                snapshot: self.start_time,
                command: Command::Disconnected,
            });
        }

        self.state = ConnectionState::Closed;

        Poll::Ready(Ok(()))
    }

    fn shutdown(&mut self) -> Poll<Result<(), ErrorKind>> {
        // If the connection active we need to notify that the player left.
        if self.mode.is_listen() && self.state == ConnectionState::Connected {
            self.queue.push(ConnectionMessage {
                id: self.id,
                snapshot: self.start_time,
                command: Command::Disconnected,
            });
        }

        let packet = Shutdown {
            reason: ShutdownReason::CLOSE,
        };

        let _ = self.send(packet, ConnectionState::Closed);
        Poll::Ready(Ok(()))
    }

    fn send<T>(&mut self, body: T, state: ConnectionState) -> Poll<()>
    where
        T: Into<PacketBody>,
    {
        let body = body.into();

        let sequence = self.next_server_sequence;
        self.next_server_sequence += 1;

        let timestamp = Timestamp::new(self.start_time.elapsed());

        let packet = Packet {
            header: Header {
                packet_type: body.packet_type(),
                sequence,
                timestamp,
            },
            body,
        };

        let socket = self.socket.clone();
        let peer = self.peer;
        self.write = Some(WriteRequest {
            future: Box::pin(async move {
                let mut buf = Vec::with_capacity(1500);
                packet.encode(&mut buf).unwrap();
                if let Err(err) = socket.send_to(&buf, peer).await {
                    tracing::error!("Failed to send packet: {}", err);
                }
            }),
            state,
        });

        Poll::Ready(())
    }
}

impl Future for Connection {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        tracing::trace!("Connection.poll");

        loop {
            if self.write.is_some() {
                match self.poll_write(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => (),
                }
            }

            match self.state {
                ConnectionState::Connected | ConnectionState::Handshake(_) => {
                    match self.poll_read(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(Error(err))),
                        Poll::Ready(Ok(())) => (),
                    }
                }
                ConnectionState::Closed => return Poll::Ready(Ok(())),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sender {
    tx: mpsc::Sender<Frame>,
}

impl Sender {
    pub fn send<T>(&self, frame: T)
    where
        T: Into<Frame>,
    {
        let _ = self.tx.try_send(frame.into());
    }
}

struct WriteRequest {
    future: Pin<Box<(dyn Future<Output = ()> + Send + Sync + 'static)>>,
    /// The state to return to once done with writing.
    state: ConnectionState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ConnectionState {
    Handshake(HandshakeState),
    Connected,
    Closed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum HandshakeState {
    ///
    /// If [`ConnectionMode::Connect`]: Hello sent, waiting for HELLO from server.
    /// If [`ConnectionMode::Listen`]: Waiting for HELLO from client.
    Hello,
    Agreement,
}

#[derive(Clone, Debug)]
pub struct FrameQueue {
    queue: VecDeque<Frame>,
}

impl FrameQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, frame: Frame) {
        self.queue.push_back(frame);
    }

    pub fn pop(&mut self) -> Option<Frame> {
        self.queue.pop_front()
    }
}

impl Extend<Frame> for FrameQueue {
    fn extend<T: IntoIterator<Item = Frame>>(&mut self, iter: T) {
        self.queue.extend(iter);
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    tx: mpsc::Sender<Packet>,
    chan_out: mpsc::Sender<Command>,
}

impl ConnectionHandle {
    pub async fn send(&self, packet: Packet) {
        self.tx.send(packet).await.unwrap();
    }

    pub fn send_cmd(&self, cmd: Command) {
        self.chan_out.try_send(cmd);
    }
}

pub struct ConnectionKey {}

/// Command backlog
///
/// Commands in the backlog are held until the entity with the given
/// id exists.
#[derive(Clone, Debug)]
pub struct Backlog {
    commands: HashMap<EntityId, Vec<Command>>,
}

impl Backlog {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: EntityId, cmd: Command) {
        match self.commands.get_mut(&id) {
            Some(vec) => vec.push(cmd),
            None => {
                self.commands.insert(id, vec![cmd]);
            }
        }
    }

    pub fn remove(&mut self, id: EntityId) -> Option<Vec<Command>> {
        self.commands.remove(&id)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionMode {
    Connect,
    Listen,
}

impl ConnectionMode {
    #[inline]
    pub const fn is_connect(self) -> bool {
        matches!(self, Self::Connect)
    }

    #[inline]
    pub const fn is_listen(self) -> bool {
        matches!(self, Self::Listen)
    }
}

#[derive(Debug)]
struct TickInterval {
    interval: tokio::time::Interval,
}

impl TickInterval {
    fn new() -> Self {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self { interval }
    }

    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.interval.poll_tick(cx).map(|_| ())
    }
}
