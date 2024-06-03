use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::BytesMut;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use game_common::world::control_frame::ControlFrame;
use game_net::conn::socket::UdpSocketStream;
use game_net::conn::{Connection, Listen};
use game_net::proto::{Decode, Error, Packet};
use game_net::socket::Socket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::conn::ConnectionKey;
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

                tracing::trace!("got {} bytes from {}", len, addr);

                let packet = match Packet::decode(&mut buf) {
                    Ok(packet) => packet,
                    Err(err) => {
                        tracing::info!("failed to decode packet: {}", err);
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
    let key = ConnectionKey { addr };

    if let Some(conn) = state.conns.get(key) {
        conn.tx().send(packet).await.unwrap();
        return;
    }

    // Unknown clients may only sent handshake requests.
    // if packet.header.packet_type != PacketType::HANDSHAKE {
    //     tracing::info!("data packet from unknown client {}", addr);
    //     return;
    // }

    let control_frame = state.control_frame.get();

    let (tx, rx) = mpsc::channel(4096);
    let stream = UdpSocketStream::new(rx, socket, addr);

    let (conn, handle) = Connection::<_, Listen>::new(stream, control_frame, ControlFrame(0));

    {
        let state = state.clone();
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                tracing::warn!("Error serving connection: {}", err);
            }

            state.conns.remove(key);
        });
    }

    tx.send(packet).await.unwrap();

    let handle = Arc::new(handle);
    state.conns.insert(key, tx, handle);
}

pub trait NewConn {
    fn recv(&mut self);
}
