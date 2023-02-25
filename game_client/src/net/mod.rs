use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use bevy::prelude::{Commands, Query, Res, ResMut, Transform, VisibilityBundle};
use game_common::bundles::ObjectBundle;
use game_common::components::object::LoadObject;
use game_common::components::player::HostPlayer;
use game_common::entity::{Entity, EntityData, EntityMap};
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
            .insert_resource(EntityMap::default())
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
            let (conn, handle) = Connection::new(addr, queue, sock.clone());
            tokio::task::spawn(async move {
                conn.await.unwrap();
            });

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

                handle.send(packet).await;
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
    mut map: ResMut<EntityMap>,
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
                    EntityKind::Object(oid) => commands
                        .spawn(
                            ObjectBundle::new(oid)
                                .translation(translation)
                                .rotation(rotation),
                        )
                        .insert(Entity {
                            id,
                            transform: Transform::from_translation(translation),
                            data: EntityData::Object { id: oid },
                        })
                        .id(),
                    EntityKind::Actor(()) => commands
                        .spawn(Transform::from_translation(translation))
                        .insert(Entity {
                            id,
                            transform: Transform::from_translation(translation),
                            data: EntityData::Actor {},
                        })
                        .id(),
                };

                map.insert(id, entity);
            }
            Command::EntityDestroy { id } => {
                let ent = map.get(id).unwrap();
                commands.entity(ent).despawn();
            }
            Command::EntityTranslate { id, translation } => {
                let entity = map.get(id).unwrap();

                if let Ok(mut transform) = entities.get_mut(entity) {
                    transform.translation = translation;
                }
            }
            Command::EntityRotate { id, rotation } => {
                let ent = map.get(id).unwrap();

                let mut transform = entities.get_mut(ent).unwrap();
                transform.rotation = rotation;
            }
            Command::SpawnHost { id } => {
                let ent = map.get(id).unwrap();

                // commands.entity(ent).insert(HostPlayer);
            }
            // Never sent to clients
            Command::PlayerJoin => (),
            Command::PlayerLeave => (),
        }
    }
}
