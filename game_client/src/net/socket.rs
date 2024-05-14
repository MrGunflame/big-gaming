use std::future::Future;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use game_common::world::control_frame::ControlFrame;
use game_net::conn::socket::UdpSocketStream;
use game_net::conn::{Connect, Connection, ConnectionHandle};
use game_net::proto::{Decode, Packet};
use game_net::Socket;
use tokio::runtime::Builder;

use super::ConnectionError;

pub fn spawn_conn(
    addr: SocketAddr,
    control_frame: ControlFrame,
    const_delay: ControlFrame,
) -> Result<ConnectionHandle, ConnectionError> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        rt.block_on(async move {
            let socket = match Socket::connect(addr) {
                Ok(s) => Arc::new(s),
                Err(err) => {
                    tx.send(Err(ConnectionError::Socket(err))).unwrap();
                    return;
                }
            };

            let (stream_tx, stream_rx) = tokio::sync::mpsc::channel(4096);
            let stream = UdpSocketStream::new(stream_rx, socket.clone(), addr);
            let (mut conn, handle) =
                Connection::<_, Connect>::new(stream, control_frame, const_delay);

            tracing::info!("connecting to {:?}", addr);

            tx.send(Ok(handle)).unwrap();

            tokio::select! {
                res = &mut conn => {
                    if let Err(err) = res {
                        tracing::error!("server error: {}", err);
                    }
                }
                _ = accept_loop(socket, stream_tx) => {}
            }

            tracing::info!("disconnected");
        });
    });

    rx.recv().unwrap()
}

fn accept_loop(
    socket: Arc<Socket>,
    tx: tokio::sync::mpsc::Sender<Packet>,
) -> impl Future<Output = Result<(), Box<dyn std::error::Error>>> {
    async move {
        loop {
            let mut buf = vec![0; 1500];
            let (len, _) = socket.recv_from(&mut buf).await?;
            buf.truncate(len);

            let packet = match Packet::decode(&buf[..]) {
                Ok(packet) => packet,
                Err(err) => {
                    tracing::warn!("failed to decode packet: {}", err);
                    continue;
                }
            };

            tx.send(packet).await.unwrap();
        }
    }
}
