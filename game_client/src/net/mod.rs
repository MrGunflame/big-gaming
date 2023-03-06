mod conn;
mod world;

use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use bevy::prelude::{Commands, Res, ResMut, Transform, Vec3};
use game_common::entity::{Entity, EntityData, EntityMap};
use game_net::conn::{Connection, ConnectionHandle, ConnectionMode};
use game_net::proto::{Decode, EntityKind, Packet};
use game_net::snapshot::{
    Command, CommandQueue, ConnectionMessage, DeltaQueue, SnapshotId, Snapshots,
};
use game_net::world::WorldState;
use game_net::Socket;
use tokio::runtime::Runtime;

pub use self::conn::ServerConnection;
use self::conn::State;

/// Client-side network plugin
#[derive(Clone, Debug, Default)]
pub struct NetPlugin {}

impl bevy::prelude::Plugin for NetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let queue = CommandQueue::new();

        let map = EntityMap::default();

        let mut snaps = Snapshots::new();
        snaps.push();

        let mut world = WorldState::new();
        world.insert(snaps.newest().unwrap());

        app.insert_resource(queue)
            .insert_resource(world)
            .insert_resource(snaps)
            .insert_resource(map.clone())
            .insert_resource(ServerConnection::new(map))
            .insert_resource(DeltaQueue::new())
            .add_system(flush_command_queue)
            .add_system(conn::update_connection_state)
            .add_system(world::apply_world_delta)
            .add_system(world::advance_snapshots)
            .add_system(world::flush_delta_queue);
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
            let (mut conn, handle) =
                Connection::new(addr, queue.clone(), sock.clone(), ConnectionMode::Connect);

            tokio::task::spawn(async move {
                if let Err(err) = (&mut conn).await {
                    tracing::error!("server error: {}", err);
                    queue.push(ConnectionMessage {
                        id: conn.id,
                        command: Command::Disconnected,
                        snapshot: SnapshotId(0),
                    });
                }
            });

            tracing::info!("connected");

            tx.send(Ok(handle.clone())).unwrap();

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
    // mut entities: Query<(&mut Transform,)>,
    // hosts: Query<bevy::ecs::entity::Entity, With<HostPlayer>>,
    mut conn: ResMut<ServerConnection>,
    map: ResMut<EntityMap>,
    mut world: ResMut<WorldState>,
    snapshots: ResMut<Snapshots>,
) {
    let Some(id) = snapshots.newest() else {
        return;
    };

    let mut view = world.get_mut(id).unwrap();

    while let Some(msg) = queue.pop() {
        dbg!(&msg);

        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                kind,
            } => {
                view.spawn(Entity {
                    id,
                    transform: Transform {
                        translation,
                        rotation,
                        scale: Vec3::splat(1.0),
                    },
                    data: match kind {
                        EntityKind::Object(id) => EntityData::Object { id },
                        EntityKind::Actor(()) => EntityData::Actor {},
                    },
                });
                // let entity = match kind {
                //     EntityKind::Object(oid) => {
                //         let id = commands
                //             .spawn(
                //                 ObjectBundle::new(oid)
                //                     .translation(translation)
                //                     .rotation(rotation),
                //             )
                //             .insert(Entity {
                //                 id,
                //                 transform: Transform::from_translation(translation),
                //                 data: EntityData::Object { id: oid },
                //             })
                //             .id();

                //         tracing::info!(
                //             "Spawning object {:?} at {:.2}, {:.2}, {:.2}",
                //             id,
                //             translation.x,
                //             translation.y,
                //             translation.z,
                //         );

                //         id
                //     }
                //     EntityKind::Actor(()) => {
                //         let mut actor = ActorBundle::default();
                //         actor.transform.transform.translation = translation;
                //         actor.transform.transform.rotation = rotation;
                //         actor.physics.collider = Collider::cuboid(1.0, 1.0, 1.0);

                //         let id = commands
                //             .spawn(actor)
                //             .insert(Entity {
                //                 id,
                //                 transform: Transform::from_translation(translation),
                //                 data: EntityData::Actor {},
                //             })
                //             .id();

                //         tracing::info!(
                //             "Spawning actor {:?} at {:.2}, {:.2}, {:.2}",
                //             id,
                //             translation.x,
                //             translation.y,
                //             translation.z,
                //         );

                //         id
                //     }
                // };

                // map.insert(id, entity);

                // Make sure the entity is spawned before processing any other
                // commands.
                // break;
            }
            Command::EntityDestroy { id } => {
                // let ent = map.get(id).unwrap();
                // commands.entity(ent).despawn();

                view.despawn(id);
            }
            Command::EntityTranslate { id, translation } => {
                // let entity = map.get(id).unwrap();

                // if let Ok((mut transform,)) = entities.get_mut(entity) {
                //     transform.translation = translation;
                // } else {
                //     tracing::warn!("unknown entity");
                // }

                let mut entity = view.get_mut(id).unwrap();
                entity.transform.translation = translation;
            }
            Command::EntityRotate { id, rotation } => {
                // if let Ok((mut transform,)) = entities.get_mut(entity) {
                // transform.rotation = rotation;
                // }

                let mut entity = view.get_mut(id).unwrap();
                entity.transform.rotation = rotation;
            }
            Command::EntityVelocity { id, linvel, angvel } => {

                // if let Ok((_,)) = entities.get_mut(entity) {
                // velocity.linvel = linvel;
                // velocity.angvel = angvel;
                // }
            }
            Command::SpawnHost { id } => {
                view.spawn_host(id);

                // let ent = map.get(id).unwrap();

                // If the world already contains a HostPlayer it must be removed.
                // Having more than one HostPlayer causes problems and must be avoided.
                // if let Ok(host) = hosts.get_single() {
                // commands.entity(host).remove::<HostPlayer>();
                // }

                // let (transform,) = entities.get(ent).unwrap();
                // tracing::info!(
                //     "Entity {:?} (located at {:.2}, {:.2}, {:.2}) is now host",
                //     ent,
                //     transform.translation.x,
                //     transform.translation.y,
                //     transform.translation.z,
                // );

                // commands
                //     .entity(ent)
                //     .insert(HostPlayer)
                //     .insert(StreamingSource::new());
            }
            // Never sent to clients
            Command::Connected => {
                conn.push_state(State::Connected);
            }
            Command::Disconnected => {
                conn.push_state(State::Disconnected);
            }
        }
    }
}
