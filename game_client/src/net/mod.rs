use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use bevy::prelude::{Commands, Query, Res, ResMut, Transform};
use game_common::bundles::{HostPlayerBundle, ObjectBundle};
use game_common::components::object::ObjectId;
use game_common::entity::{Entity, EntityData, EntityMap};
use game_net::conn::{Connection, ConnectionHandle};
use game_net::proto::{Decode, EntityKind, Packet};
use game_net::snapshot::{Command, CommandQueue};
use game_net::Socket;
use tokio::runtime::Runtime;

pub use self::conn::ServerConnection;

mod conn;

/// Client-side network plugin
#[derive(Clone, Debug, Default)]
pub struct NetPlugin {}

impl bevy::prelude::Plugin for NetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let queue = CommandQueue::new();

        let map = EntityMap::default();

        app.insert_resource(queue)
            .insert_resource(map.clone())
            .insert_resource(ServerConnection::stub(map))
            .add_system(flush_command_queue)
            .add_system(conn::update_connection_state);
    }
}

pub fn spawn_conn(
    queue: CommandQueue,
    addr: SocketAddr,
) -> Result<ConnectionHandle, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let sock = match Socket::connect(addr) {
                Ok(s) => Arc::new(s),
                Err(err) => {
                    tx.send(Err(err.into())).unwrap();
                    return;
                }
            };
            let (conn, handle) = Connection::new(addr, queue, sock.clone());

            tokio::task::spawn(async move {
                conn.await.unwrap();
            });

            tracing::info!("connected");

            tx.send(Ok(handle.clone())).unwrap();

            handle.send_cmd(Command::PlayerJoin);

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
                        .spawn(
                            ObjectBundle::new(ObjectId(0.into()))
                                .translation(translation)
                                .rotation(rotation),
                        )
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
                let entity = map.get(id).unwrap();

                if let Ok(mut transform) = entities.get_mut(entity) {
                    transform.rotation = rotation;
                }
            }
            Command::SpawnHost { id } => {
                let ent = map.get(id).unwrap();

                commands.entity(ent).insert(HostPlayerBundle::new());
            }
            // Never sent to clients
            Command::PlayerJoin => (),
            Command::PlayerLeave => (),
        }
    }
}
