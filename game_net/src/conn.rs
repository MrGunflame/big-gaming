use std::collections::VecDeque;
use std::future::Future;
use std::net::UdpSocket;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};

use futures::Sink;
use pin_project::pin_project;
use tokio::sync::mpsc;

use crate::proto::{Error, Frame, Header, Packet, PacketType};

pub struct Connection {
    /// Input stream from the socket
    stream: mpsc::Receiver<Packet>,
    writer: mpsc::Receiver<Frame>,
    reader: mpsc::Sender<Frame>,
    state: ConnectionState,

    queue: FrameQueue,
    sink: mpsc::Sender<Packet>,
}

impl Connection {
    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        #[cfg(debug_assertions)]
        assert!(matches!(self.state, ConnectionState::Read));

        if let Poll::Ready(packet) = self.stream.poll_recv(cx) {
            let packet = packet.unwrap();

            for frame in packet.frames {
                self.reader.try_send(frame);
            }
        }

        if let Poll::Ready(frame) = self.writer.poll_recv(cx) {
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

            self.sink.try_send(packet).unwrap();
        }

        self.state = ConnectionState::Read;
        Poll::Ready(())
    }
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
