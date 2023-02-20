use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::BytesMut;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use game_net::conn::Connection;
use game_net::proto::{Decode, Error, Packet, PacketType};
use game_net::socket::Socket;
use tokio::task::JoinHandle;

use crate::state::State;

pub struct Server {
    workers: FuturesUnordered<Worker>,
}

impl Server {
    pub fn new(state: State) -> Result<Self, io::Error> {
        let socket = Arc::new(Socket::bind("0.0.0.0:6942")?);

        tracing::info!("listening on {}", "0.0.0.0:6942");

        let workers = FuturesUnordered::new();
        for id in 0..1 {
            let worker = Worker::new(id, socket.clone(), state.clone());
            workers.push(worker);
        }

        Ok(Self { workers })
    }
}

impl Future for Server {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.workers.poll_next_unpin(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Err(err)),
                _ => (),
            }
        }
    }
}

struct Worker {
    handle: JoinHandle<Result<(), Error>>,
}

impl Worker {
    pub fn new(id: usize, socket: Arc<Socket>, state: State) -> Self {
        let handle = tokio::task::spawn(async move {
            tracing::info!("spawned worker thread {}", id);

            loop {
                let mut buf = BytesMut::zeroed(1500);
                let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
                buf.truncate(len);

                let packet = match Packet::decode(&mut buf) {
                    Ok(packet) => packet,
                    Err(err) => {
                        tracing::debug!("failed to decode packet: {}", err);
                        continue;
                    }
                };

                handle_packet(addr, socket.clone(), &state, packet).await;
            }
        });

        Self { handle }
    }
}

impl Future for Worker {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.handle.poll_unpin(cx).map(|res| res.unwrap())
    }
}

async fn handle_packet(addr: SocketAddr, socket: Arc<Socket>, state: &State, packet: Packet) {
    if let Some(handle) = state.pool.get(addr) {
        handle.send(packet);
        return;
    }

    // Unknown clients may only sent handshake requests.
    if packet.header.packet_type != PacketType::HANDSHAKE {
        return;
    }

    let handle = Connection::new(addr, state.queue.clone(), socket);
    handle.send(packet);

    state.pool.insert(addr, handle);
}
