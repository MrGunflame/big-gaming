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

use crate::buffer::FrameBuffer;
use crate::entity::Entities;
use crate::proto::ack::{Ack, Nak};
use crate::proto::handshake::{Handshake, HandshakeFlags, HandshakeType};
use crate::proto::sequence::Sequence;
use crate::proto::shutdown::{Shutdown, ShutdownReason};
use crate::proto::timestamp::Timestamp;
use crate::proto::{Encode, Frame, Header, Packet, PacketBody, PacketType};
use crate::request::Request;
use crate::snapshot::{Command, CommandId, CommandQueue, ConnectionMessage, Response, Status};
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
    chan_out: mpsc::Receiver<ConnectionMessage>,

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

    commands: Commands,
    buffer: FrameBuffer,
    write_queue: WriteQueue,
    ack_list: AckList,
}

impl Connection {
    pub fn new(
        peer: SocketAddr,
        queue: CommandQueue,
        socket: Arc<Socket>,
        mode: ConnectionMode,
    ) -> (Self, ConnectionHandle) {
        let id = ConnectionId::new();

        let (tx, rx) = mpsc::channel(4096);
        let (out_tx, out_rx) = mpsc::channel(4096);

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
            commands: Commands {
                cmds: Default::default(),
            },
            buffer: FrameBuffer::new(),
            write_queue: WriteQueue::new(),
            ack_list: AckList::default(),
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
                next_id: Arc::new(AtomicU32::new(0)),
            },
        )
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ErrorKind>> {
        tracing::trace!("Connection.poll_read");

        // Flush the send buffer before reading any packets.
        if !self.write_queue.is_empty() {
            self.init_write(self.state);
            return Poll::Ready(Ok(()));
        }

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

        while let Poll::Ready(msg) = self.chan_out.poll_recv(cx) {
            let Some(msg) = msg else {
                return self.shutdown();
            };

            match &msg.command {
                // The game loop has processed the commands.
                // We can now acknowledge that we have processed the commands.
                Command::ReceivedCommands { ids } => {
                    for resp in ids {
                        let seq = self.ack_list.list.remove(&resp.id).unwrap();
                        // The last sequence acknowledged by the game loop.
                        self.ack_list.ack_seq = seq;
                    }

                    continue;
                }
                _ => (),
            }

            let msgid = msg.id.unwrap();

            let frame = match self.entities.pack(&msg.command) {
                Some(frame) => frame,
                None => {
                    tracing::info!(
                        "backlogging command {:?} for entity {:?}",
                        msg,
                        msg.command.id()
                    );

                    if let Some(id) = msg.command.id() {
                        self.backlog.insert(id, msg.command);
                    }

                    continue;
                }
            };

            self.frame_queue.push(frame, msgid, msg.snapshot);

            if let Some(id) = msg.command.id() {
                if let Some(vec) = self.backlog.remove(id) {
                    tracing::info!("flushing commands for {:?}", id);

                    self.frame_queue.extend(
                        vec.into_iter()
                            .map(|cmd| (self.entities.pack(&cmd).unwrap(), msgid, msg.snapshot)),
                    );
                }
            }
        }

        Poll::Pending
    }

    fn write_snapshot(&mut self) {
        // Merge FrameQueue into FrameBuffer, compact then send.

        let timestamp = Timestamp::new(self.start_time.elapsed());

        while let Some((frame, id, snapshot)) = self.frame_queue.pop() {
            let sequence = self.next_server_sequence;
            self.next_server_sequence += 1;

            self.buffer.push(Request {
                id,
                sequence,
                frame,
            });
        }

        // for req in self.buffer.compact() {
        //     self.queue.push(ConnectionMessage {
        //         id: None,
        //         conn: self.id,
        //         snapshot: Instant::now(),
        //         command: Command::ReceivedCommands {
        //             ids: vec![Response {
        //                 id: req.id,
        //                 status: Status::Overwritten,
        //             }],
        //         },
        //     });
        // }

        for req in &self.buffer {
            let sequence = self.next_server_sequence;
            self.next_server_sequence += 1;

            self.commands.cmds.insert(sequence, vec![req.id]);

            let packet = Packet {
                header: Header {
                    packet_type: PacketType::DATA,
                    sequence,
                    timestamp,
                },
                body: PacketBody::Frames(vec![req.frame.clone()]),
            };

            self.write_queue.push(packet);
        }

        self.buffer.clear();
    }

    fn init_write(&mut self, state: ConnectionState) {
        let socket = self.socket.clone();
        let peer = self.peer;

        let packet = self.write_queue.pop().unwrap();

        self.write = Some(WriteRequest {
            future: Box::pin(async move {
                let mut buf = Vec::with_capacity(1500);
                packet.encode(&mut buf).unwrap();
                if let Err(err) = socket.send_to(&buf, peer).await {
                    tracing::error!("failed to send packet: {}", err);
                }
            }),
            state,
        });
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
        // TODO: Handle out-of-order / duplicates / drops
        self.next_peer_sequence += 1;

        let snapshot = self.start_time + header.timestamp.to_duration();

        for frame in frames {
            let Some(cmd) = self.entities.unpack(frame) else {
                    tracing::debug!("failed to translate cmd");
                    continue;
                };

            let msgid = self.ack_list.next_cmd_id;
            self.ack_list.next_cmd_id.0 += 1;

            self.ack_list.list.insert(msgid, header.sequence);

            self.queue.push(ConnectionMessage {
                id: Some(msgid),
                conn: self.id,
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
                    id: None,
                    conn: self.id,
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
                    id: None,
                    conn: self.id,
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
        let sequence = body.sequence;

        if let Some(ids) = self.commands.cmds.remove(&sequence) {
            self.queue.push(ConnectionMessage {
                id: None,
                conn: self.id,
                snapshot: Instant::now(),
                command: Command::ReceivedCommands {
                    ids: ids
                        .into_iter()
                        .map(|id| Response {
                            id,
                            status: Status::Received,
                        })
                        .collect(),
                },
            });
        }

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
        let tick = ready!(self.interval.poll_tick(cx));

        if self.last_time.elapsed() >= Duration::from_secs(15) {
            tracing::info!("closing connection due to timeout");

            self.shutdown();
            return Poll::Ready(Err(ErrorKind::TimedOut));
        }

        // Send periodic ACKs while connected.
        if self.state == ConnectionState::Connected && tick.is_ack() {
            let _ = self.send(
                Ack {
                    sequence: self.ack_list.ack_seq,
                },
                ConnectionState::Connected,
            );
            Poll::Ready(Ok(()))
        } else if self.state == ConnectionState::Connected && tick.is_fire() {
            self.write_snapshot();
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
                id: None,
                conn: self.id,
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
                id: None,
                conn: self.id,
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

    fn send_data(
        &mut self,
        body: Vec<Frame>,
        cmds: Vec<CommandId>,
        snapshot: Instant,
        state: ConnectionState,
    ) -> Poll<()> {
        let sequence = self.next_server_sequence;
        self.next_server_sequence += 1;

        self.commands.cmds.insert(sequence, cmds);

        let timestamp = Timestamp::new(snapshot - self.start_time);

        let packet = Packet {
            header: Header {
                packet_type: PacketType::DATA,
                sequence,
                timestamp,
            },
            body: PacketBody::Frames(body),
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
    queue: VecDeque<(Frame, CommandId, Instant)>,
}

impl FrameQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, frame: Frame, id: CommandId, ts: Instant) {
        self.queue.push_back((frame, id, ts));
    }

    pub fn pop(&mut self) -> Option<(Frame, CommandId, Instant)> {
        self.queue.pop_front()
    }
}

impl Extend<(Frame, CommandId, Instant)> for FrameQueue {
    fn extend<T: IntoIterator<Item = (Frame, CommandId, Instant)>>(&mut self, iter: T) {
        self.queue.extend(iter);
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    tx: mpsc::Sender<Packet>,
    chan_out: mpsc::Sender<ConnectionMessage>,
    next_id: Arc<AtomicU32>,
}

impl ConnectionHandle {
    pub async fn send(&self, packet: Packet) {
        self.tx.send(packet).await.unwrap();
    }

    pub fn send_cmd(&self, mut cmd: ConnectionMessage) -> CommandId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        cmd.id = Some(CommandId(id));

        self.chan_out.try_send(cmd);
        CommandId(id)
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
    /// Total number of commands pushed.
    ///
    /// Only used when `debug_assertions` is enabled: When pushing too many commands without ever
    /// removing them, [`insert`] will panic above the threshold [`MAX_LEN`].
    ///
    /// [`insert`]: Self::insert
    /// [`MAX_LEN`]: Self::MAX_LEN
    #[cfg(debug_assertions)]
    len: usize,
}

impl Backlog {
    #[cfg(debug_assertions)]
    const MAX_LEN: usize = 8192;

    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            #[cfg(debug_assertions)]
            len: 0,
        }
    }

    pub fn insert(&mut self, id: EntityId, cmd: Command) {
        #[cfg(debug_assertions)]
        {
            self.len += 1;
            if self.len > Self::MAX_LEN {
                panic!("exceeded maximum backlog len of {} commands", Self::MAX_LEN);
            }
        }

        match self.commands.get_mut(&id) {
            Some(vec) => vec.push(cmd),
            None => {
                self.commands.insert(id, vec![cmd]);
            }
        }
    }

    pub fn remove(&mut self, id: EntityId) -> Option<Vec<Command>> {
        match self.commands.remove(&id) {
            Some(cmds) => {
                #[cfg(debug_assertions)]
                {
                    self.len -= cmds.len();
                }

                Some(cmds)
            }
            None => None,
        }
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
    tick: u16,
}

impl TickInterval {
    fn new() -> Self {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self { interval, tick: 0 }
    }

    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Tick> {
        self.interval.poll_tick(cx).map(|_| {
            let tick = Tick { tick: self.tick };
            self.tick += 1;
            tick
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct Tick {
    tick: u16,
}

impl Tick {
    fn is_ack(&self) -> bool {
        (self.tick & 10) == 0
    }

    fn is_fire(&self) -> bool {
        true
    }
}

struct Commands {
    cmds: HashMap<Sequence, Vec<CommandId>>,
}

struct WriteQueue {
    packets: VecDeque<Packet>,
}

impl WriteQueue {
    #[inline]
    fn new() -> Self {
        Self {
            packets: VecDeque::new(),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.packets.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn push(&mut self, packet: Packet) {
        self.packets.push_back(packet);
    }

    #[inline]
    fn pop(&mut self) -> Option<Packet> {
        self.packets.pop_front()
    }
}

#[derive(Debug, Default)]
pub struct AckList {
    list: HashMap<CommandId, Sequence>,
    next_cmd_id: CommandId,
    ack_seq: Sequence,
}
