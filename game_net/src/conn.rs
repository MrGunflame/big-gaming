use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::FutureExt;
use game_common::entity::EntityId;
use tokio::sync::mpsc;

use crate::entity::Entities;
use crate::proto::handshake::{Handshake, HandshakeFlags, HandshakeType};
use crate::proto::{Encode, Error, Frame, Header, Packet, PacketBody, PacketType};
use crate::snapshot::{Command, CommandQueue, ConnectionMessage};
use crate::socket::Socket;

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
    id: ConnectionId,
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
}

impl Connection {
    pub fn new(
        peer: SocketAddr,
        queue: CommandQueue,
        socket: Arc<Socket>,
    ) -> (Self, ConnectionHandle) {
        let id = ConnectionId::new();

        let (tx, rx) = mpsc::channel(32);
        let (out_tx, out_rx) = mpsc::channel(32);

        let conn = Self {
            id,
            stream: rx,
            socket,
            state: ConnectionState::Read,
            chan_out: out_rx,
            queue,
            entities: Entities::new(),
            peer,
            backlog: Backlog::new(),
            frame_queue: FrameQueue::new(),
            mode: ConnectionMode::Listen,
            write: None,
        };

        (
            conn,
            ConnectionHandle {
                id,
                tx,
                chan_out: out_tx,
            },
        )
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        tracing::trace!("Connection.poll_read");

        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Read));

        while let Poll::Ready(packet) = self.stream.poll_recv(cx) {
            tracing::debug!("recv {:?}", packet);

            let packet = packet.unwrap();

            let frames = match packet.body {
                PacketBody::Frames(f) => f,
                _ => unreachable!(),
            };

            for frame in frames {
                let Some(cmd) = self.entities.unpack(frame) else {
                    tracing::debug!("failed to translate cmd");
                    continue;
                };

                self.queue.push(ConnectionMessage {
                    id: self.id,
                    command: cmd,
                });
            }
        }

        while let Poll::Ready(cmd) = self.chan_out.poll_recv(cx) {
            let cmd = cmd.unwrap();

            let frame = match self.entities.pack(&cmd) {
                Some(frame) => frame,
                None => {
                    tracing::info!("backlogging command");

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
            let socket = self.socket.clone();

            let packet = Packet {
                header: Header {
                    packet_type: PacketType::DATA,
                    timestamp: 0,
                    sequence_number: 0,
                    _resv0: 0,
                },
                body: PacketBody::Frames(vec![frame]),
            };

            let peer = self.peer;
            self.write = Some(WriteRequest {
                future: Box::pin(async move {
                    let mut buf = Vec::with_capacity(1500);
                    packet.encode(&mut buf).unwrap();

                    // tracing::info!("sending {:?} ({} bytes)", packet, buf.len());

                    socket.send_to(&buf, peer).await.unwrap();
                }),
                state: ConnectionState::Read,
            });

            return Poll::Ready(());
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

    fn poll_handshake(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        tracing::trace!("Connection.poll_handshake");

        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Handshake(_)));

        let ConnectionState::Handshake(state) = self.state else {
            unreachable!();
        };

        let Poll::Ready(packet) = self.stream.poll_recv(cx) else {
            return Poll::Pending;
        };
        let packet = packet.unwrap();

        match state {
            // Connect mode
            HandshakeState::Hello if self.mode == ConnectionMode::Connect => {
                let body = match packet.body {
                    PacketBody::Handshake(body) => body,
                    _ => panic!("expected HS"),
                };

                // Send AGREEMENT
                let packet = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        _resv0: 0,
                        sequence_number: 0,
                        timestamp: 0,
                    },
                    body: Handshake {
                        version: 0,
                        kind: HandshakeType::AGREEMENT,
                        flags: HandshakeFlags::default(),
                        mtu: 1500,
                        flow_window: 8192,
                    }
                    .into(),
                };

                let socket = self.socket.clone();
                let peer = self.peer;
                self.write = Some(WriteRequest {
                    future: Box::pin(async move {
                        let mut buf = Vec::with_capacity(1500);
                        packet.encode(&mut buf).unwrap();
                        socket.send_to(&buf, peer).await.unwrap();
                    }),
                    state: ConnectionState::Handshake(HandshakeState::Agreement),
                });
            }
            HandshakeState::Agreement if self.mode == ConnectionMode::Connect => {
                let body = match packet.body {
                    PacketBody::Handshake(body) => body,
                    _ => panic!("err"),
                };

                self.state = ConnectionState::Read;
            }
            // Listen mode
            HandshakeState::Hello if self.mode == ConnectionMode::Listen => {
                assert!(matches!(packet.body, PacketBody::Handshake(_)));

                // Send HELLO
                let packet = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        _resv0: 0,
                        sequence_number: 0,
                        timestamp: 0,
                    },
                    body: Handshake {
                        version: 0,
                        kind: HandshakeType::HELLO,
                        flags: HandshakeFlags::default(),
                        mtu: 1500,
                        flow_window: 8192,
                    }
                    .into(),
                };

                let socket = self.socket.clone();
                let peer = self.peer;
                self.write = Some(WriteRequest {
                    future: Box::pin(async move {
                        let mut buf = Vec::with_capacity(1500);
                        packet.encode(&mut buf).unwrap();
                        socket.send_to(&buf, peer).await.unwrap();
                    }),
                    state: ConnectionState::Handshake(HandshakeState::Agreement),
                });
            }
            HandshakeState::Agreement if self.mode == ConnectionMode::Listen => {
                let packet = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        _resv0: 0,
                        sequence_number: 0,
                        timestamp: 0,
                    },
                    body: Handshake {
                        version: 0,
                        kind: HandshakeType::AGREEMENT,
                        flags: HandshakeFlags::default(),
                        mtu: 1500,
                        flow_window: 8192,
                    }
                    .into(),
                };

                let socket = self.socket.clone();
                let peer = self.peer;
                self.write = Some(WriteRequest {
                    future: Box::pin(async move {
                        let mut buf = Vec::with_capacity(1500);
                        packet.encode(&mut buf).unwrap();
                        socket.send_to(&buf, peer).await.unwrap();
                    }),
                    state: ConnectionState::Handshake(HandshakeState::Agreement),
                });
            }
            _ => unreachable!(),
        }

        Poll::Ready(())
    }

    fn handle_packet(&mut self, packet: Packet) {
        match packet.body {
            PacketBody::Handshake(body) => {}
            PacketBody::Frames(body) => {}
            _ => (),
        }
    }

    fn handle_handshake(&mut self, packet: Packet) {}
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
                ConnectionState::Handshake(_) => match self.poll_handshake(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => (),
                },
                ConnectionState::Read => match self.poll_read(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => (),
                },
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
    Read,
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
        self.chan_out.try_send(cmd).unwrap();
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
