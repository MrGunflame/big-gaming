use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc};

use crate::entity::Entities;
use crate::proto::{Error, Frame, Header, Packet, PacketType};
use crate::socket::Socket;

pub struct Connection {
    /// Input stream from the socket
    stream: mpsc::Receiver<Packet>,
    socket: Arc<Socket>,

    /// Direction (from self)
    chan_out: mpsc::Receiver<Frame>,
    chan_in: broadcast::Sender<Frame>,

    state: ConnectionState,

    queue: FrameQueue,
    entities: Arc<RwLock<Entities>>,
}

impl Connection {
    pub fn new(socket: Arc<Socket>) -> ConnectionHandle {
        let entities: Arc<RwLock<Entities>> = Arc::default();

        let (tx, rx) = mpsc::channel(32);
        let (in_tx, in_rx) = broadcast::channel(32);
        let (out_tx, out_rx) = mpsc::channel(32);

        let conn = Self {
            stream: rx,
            socket,
            state: ConnectionState::Read,
            chan_in: in_tx,
            chan_out: out_rx,
            queue: FrameQueue::new(),
            entities: entities.clone(),
        };

        tokio::task::spawn(async move {
            conn.await.unwrap();
        });

        ConnectionHandle {
            tx,
            chan_in: in_rx,
            chan_out: out_tx,
            entities,
        }
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Read));

        if let Poll::Ready(packet) = self.stream.poll_recv(cx) {
            let packet = packet.unwrap();

            for frame in packet.frames {
                self.chan_in.send(frame);
            }
        }

        if let Poll::Ready(frame) = self.chan_out.poll_recv(cx) {
            let frame = frame.unwrap();
            self.queue.push(frame);
        }

        Poll::Pending
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Write));

        while let Some(frame) = self.queue.pop() {
            let packet = Packet {
                header: Header {
                    packet_type: PacketType::FRAME,
                    timestamp: 0,
                    sequence_number: 0,
                },
                frames: vec![frame],
            };

            // self.sink.try_send(packet).unwrap();
        }

        self.state = ConnectionState::Read;
        Poll::Ready(())
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
        loop {
            match self.state {
                ConnectionState::Read => match self.poll_read(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => (),
                },
                ConnectionState::Write => match self.poll_write(cx) {
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
    Write,
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

#[derive(Debug)]
pub struct ConnectionHandle {
    tx: mpsc::Sender<Packet>,
    chan_in: broadcast::Receiver<Frame>,
    chan_out: mpsc::Sender<Frame>,
    entities: Arc<RwLock<Entities>>,
}

impl ConnectionHandle {
    pub async fn send(&self, packet: Packet) {
        let _ = self.tx.send(packet).await;
    }
}

impl Clone for ConnectionHandle {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            chan_out: self.chan_out.clone(),
            chan_in: self.chan_in.resubscribe(),
            entities: self.entities.clone(),
        }
    }
}
