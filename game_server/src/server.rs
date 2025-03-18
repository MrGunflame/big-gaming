use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use ahash::HashMap;
use bytes::BytesMut;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use game_common::world::control_frame::ControlFrame;
use game_net::conn::socket::UdpSocketStream;
use game_net::conn::{Connection, ConnectionStream, Listen};
use game_net::proto::{Decode, Error, Packet};
use game_net::socket::Socket;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::conn::ConnectionKey;
use crate::state::State;

struct ServerState {
    socket: Arc<Socket>,
    conns: RwLock<HashMap<SocketAddr, mpsc::Sender<Packet>>>,
    pool: ConnectionPool,
}

pub struct Server {
    workers: FuturesUnordered<Worker>,
}

impl Server {
    pub fn new(pool: ConnectionPool) -> Result<Self, io::Error> {
        let socket = Arc::new(Socket::bind("0.0.0.0:6942")?);

        tracing::info!("listening on {}", "0.0.0.0:6942");

        let state = Arc::new(ServerState {
            socket,
            conns: RwLock::default(),
            pool,
        });
        let workers = FuturesUnordered::new();
        for id in 0..1 {
            let worker = Worker::new(id, state.clone());
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
    pub fn new(id: usize, state: Arc<ServerState>) -> Self {
        let handle = tokio::task::spawn(async move {
            tracing::info!("spawned worker thread {}", id);

            let local_addr = state.socket.local_addr().unwrap();

            loop {
                let mut buf = BytesMut::zeroed(1500);
                let (len, addr) = state.socket.recv_from(&mut buf).await.unwrap();
                buf.truncate(len);

                tracing::trace!("got {} bytes from {}", len, addr);

                let packet = match Packet::decode(&mut buf) {
                    Ok(packet) => packet,
                    Err(err) => {
                        tracing::info!("failed to decode packet: {}", err);
                        continue;
                    }
                };

                handle_packet(local_addr, addr, &state, packet).await;
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

async fn handle_packet(
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    state: &ServerState,
    packet: Packet,
) {
    let key = ConnectionKey {
        local_addr,
        remote_addr,
    };

    // Clone the sender and don't borrow it over the
    // await point.
    let tx = {
        if let Some(tx) = state.conns.read().get(&remote_addr) {
            Some(tx.clone())
        } else {
            None
        }
    };

    if let Some(tx) = tx {
        tx.send(packet).await.unwrap();
        return;
    }

    // Unknown clients may only sent handshake requests.
    // if packet.header.packet_type != PacketType::HANDSHAKE {
    //     tracing::info!("data packet from unknown client {}", addr);
    //     return;
    // }

    let (tx, rx) = mpsc::channel(4096);
    let stream = UdpSocketStream::new(rx, state.socket.clone(), remote_addr);
    state.pool.spawn(key, stream);

    tx.send(packet).await.unwrap();
    state.conns.write().insert(remote_addr, tx);
}

#[derive(Debug)]
pub struct ConnectionPool {
    state: State,
}

impl ConnectionPool {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn spawn<S>(&self, key: ConnectionKey, stream: S)
    where
        S: ConnectionStream + Send + 'static,
        S::Error: std::error::Error,
    {
        let (conn, handle) = Connection::<_, Listen>::new(
            stream,
            self.state.control_frame.get(),
            key.local_addr,
            key.remote_addr,
        );

        let state = self.state.clone();
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                tracing::warn!("Error serving connection: {}", err);
            }

            state.conns.remove(key);
        });

        let handle = Arc::new(handle);
        self.state.conns.insert(key, handle);
    }
}
