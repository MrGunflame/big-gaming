use std::collections::VecDeque;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::FutureExt;
use tokio::sync::mpsc;

use crate::entity::Entities;
use crate::proto::{Encode, Error, Frame, Header, Packet, PacketType};
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
}

impl Connection {
    pub fn new(peer: SocketAddr, queue: CommandQueue, socket: Arc<Socket>) -> ConnectionHandle {
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
        };

        tokio::task::spawn(async move {
            conn.await.unwrap();
        });

        ConnectionHandle {
            id,
            tx,
            chan_out: out_tx,
        }
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        tracing::trace!("Connection.poll_read");

        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Read));

        if let Poll::Ready(packet) = self.stream.poll_recv(cx) {
            let packet = packet.unwrap();

            for frame in packet.frames {
                let Some(cmd) = self.entities.translate(frame) else {
                    tracing::debug!("failed to translate cmd");
                    continue;
                };

                self.queue.push(ConnectionMessage {
                    id: self.id,
                    command: cmd,
                });
            }
        }

        if let Poll::Ready(cmd) = self.chan_out.poll_recv(cx) {
            let cmd = cmd.unwrap();

            if let Command::RegisterEntity { id, entity } = cmd {
                self.entities.insert(entity, id);
                return Poll::Ready(());
            }

            tracing::info!("sending {:?}", cmd);

            let socket = self.socket.clone();

            let frame = self.entities.translate_cmd(cmd).unwrap();

            let packet = Packet {
                header: Header {
                    packet_type: PacketType::DATA,
                    timestamp: 0,
                    sequence_number: 0,
                },
                frames: vec![frame],
            };

            let peer = self.peer;
            self.state = ConnectionState::Write(Box::pin(async move {
                let mut buf = Vec::with_capacity(1500);
                packet.encode(&mut buf).unwrap();

                tracing::info!("sending {:?} ({} bytes)", packet, buf.len());

                socket.send_to(&buf, peer).await.unwrap();
            }));

            return Poll::Ready(());
        }

        Poll::Pending
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        tracing::trace!("Connection.poll_write");

        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Write(_)));

        let fut = match &mut self.state {
            ConnectionState::Write(fut) => fut,
            _ => unreachable!(),
        };

        match fut.poll_unpin(cx) {
            Poll::Ready(_) => {
                self.state = ConnectionState::Read;
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn handle_packet(&mut self, packet: Packet) {
        match packet.header.packet_type {
            PacketType::HANDSHAKE => self.handle_handshake(packet),
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
            match self.state {
                ConnectionState::Read => match self.poll_read(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => (),
                },
                ConnectionState::Write(_) => match self.poll_write(cx) {
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

enum ConnectionState {
    Read,
    Write(Pin<Box<(dyn Future<Output = ()> + Send + Sync + 'static)>>),
    Closed,
}

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

#[derive(Clone, Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    tx: mpsc::Sender<Packet>,
    chan_out: mpsc::Sender<Command>,
}

impl ConnectionHandle {
    pub fn send(&self, packet: Packet) {
        self.tx.try_send(packet).unwrap();
    }

    pub fn send_cmd(&self, cmd: Command) {
        self.chan_out.try_send(cmd).unwrap();
    }
}

pub struct ConnectionKey {}
