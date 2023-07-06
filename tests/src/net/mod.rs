use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use game_common::world::control_frame::ControlFrame;
use game_net::conn::{Connect, Connection, ConnectionHandle, ConnectionMode};
use game_net::proto::{Decode, Packet};
use game_net::snapshot::CommandQueue;
use game_net::Socket;

pub mod hello;
pub mod overrides;

fn connect(queue: CommandQueue) -> ConnectionHandle {
    let addr = SocketAddr::from_str("127.0.0.1:6942").unwrap();

    let socket = Arc::new(Socket::connect("127.0.0.1:6942").unwrap());

    let (conn, handle) = Connection::<Connect>::new(
        addr,
        queue,
        socket.clone(),
        ControlFrame(0),
        ControlFrame(0),
    );

    {
        let handle = handle.clone();
        tokio::task::spawn(async move {
            loop {
                let mut buf = vec![0; 1500];
                let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
                buf.truncate(len);

                let packet = Packet::decode(&buf[..]).unwrap();
                handle.send(packet).await;
            }
        });
    }

    tokio::task::spawn(async move {
        conn.await.unwrap();
    });

    handle
}
