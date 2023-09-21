use std::future::Future;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use game_common::world::control_frame::ControlFrame;
use game_net::conn::{Connect, Connection, ConnectionHandle};
use game_net::proto::{Decode, Packet};
use game_net::Socket;
use tokio::runtime::{Builder, UnhandledPanic};

pub fn spawn_conn(
    addr: SocketAddr,
    control_frame: ControlFrame,
    const_delay: ControlFrame,
) -> Result<Arc<ConnectionHandle>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = Builder::new_current_thread()
            .enable_all()
            .unhandled_panic(UnhandledPanic::ShutdownRuntime)
            .build()
            .unwrap();

        rt.block_on(async move {
            let socket = match Socket::connect(addr) {
                Ok(s) => Arc::new(s),
                Err(err) => {
                    tx.send(Err(err.into())).unwrap();
                    return;
                }
            };
            let (mut conn, handle) =
                Connection::<Connect>::new(addr, socket.clone(), control_frame, const_delay);

            tracing::info!("connected");

            let handle = Arc::new(handle);

            tx.send(Ok(handle.clone())).unwrap();

            tokio::select! {
                res = &mut conn => {
                    if let Err(err) = res {
                        tracing::error!("server error: {}", err);
                    }
                }
                _ = accept_loop(socket, handle) => {}
            }

            tracing::info!("disconnected");
        });
    });

    rx.recv().unwrap()
}

fn accept_loop(
    socket: Arc<Socket>,
    handle: Arc<ConnectionHandle>,
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

            handle.send(packet).await;
        }
    }
}
