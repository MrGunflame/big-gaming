use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use bevy::prelude::{Commands, Query, Res, Transform, VisibilityBundle};
use game_common::components::object::LoadObject;
use game_common::components::player::HostPlayer;
use game_net::conn::{Connection, ConnectionHandle};
use game_net::proto::{Decode, EntityKind, Packet};
use game_net::snapshot::{Command, CommandQueue};
use game_net::Socket;
use tokio::runtime::Runtime;

use self::conn::ServerConnection;

mod conn;

/// Client-side network plugin
#[derive(Clone, Debug, Default)]
pub struct NetPlugin {}

impl bevy::prelude::Plugin for NetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let queue = CommandQueue::new();

        let handle = spawn_conn(queue.clone());

        handle.send_cmd(Command::PlayerJoin);

        app.insert_resource(queue)
            .insert_resource(ServerConnection::new(handle))
            .add_system(flush_command_queue);
    }
}

fn spawn_conn(queue: CommandQueue) -> ConnectionHandle {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let addr = SocketAddr::from(([127, 0, 0, 1], 6942));

            let sock = Arc::new(Socket::connect(addr).unwrap());
            let handle = Connection::new(addr, queue, sock.clone());

            tracing::info!("connected");

            tx.send(handle.clone()).unwrap();

            loop {
                let mut buf = vec![0; 1500];
                let (len, addr) = sock.recv_from(&mut buf).await.unwrap();
                buf.truncate(len);

                let packet = match Packet::decode(&buf[..]) {
                    Ok(packet) => packet,
                    Err(err) => {
                        tracing::warn!("failed to decode packet: {}", err);
                        continue;
                    }
                };

                handle.send(packet);
            }
        });
    });

    rx.recv().unwrap()
}

fn flush_command_queue(
    mut commands: Commands,
    queue: Res<CommandQueue>,
    mut entities: Query<&mut Transform>,
    mut conn: Res<ServerConnection>,
) {
    while let Some(msg) = queue.pop() {
        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                kind,
            } => {
                let entity = match kind {
                    EntityKind::Object(id) => commands
                        .spawn(Transform::from_translation(translation))
                        .insert(VisibilityBundle::default())
                        .insert(LoadObject { id })
                        .id(),
                    EntityKind::Actor(()) => commands
                        .spawn(Transform::from_translation(translation))
                        .id(),
                };

                conn.send(Command::RegisterEntity { id, entity });
            }
            Command::EntityDestroy { id } => {
                commands.entity(id).despawn();
            }
            Command::EntityTranslate { id, translation } => {
                let mut transform = entities.get_mut(id).unwrap();
                transform.translation = translation;
            }
            Command::EntityRotate { id, rotation } => {
                let mut transform = entities.get_mut(id).unwrap();
                transform.rotation = rotation;
            }
            Command::SpawnHost { id } => {
                commands.entity(id).insert(HostPlayer);
            }
            // Never sent to clients
            Command::PlayerJoin => (),
            Command::PlayerLeave => (),
            Command::RegisterEntity { id, entity } => unreachable!(),
        }
    }
}
