mod conn;
mod interpolate;
mod prediction;
mod world;

use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use bevy::prelude::{dbg, IntoSystemConfig, Res, ResMut, SystemSet, Transform, Vec3};
use game_common::components::components::Components;
use game_common::entity::EntityMap;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::world::WorldState;
use game_net::backlog::Backlog;
use game_net::conn::{Connection, ConnectionHandle, ConnectionMode};
use game_net::proto::{Decode, Packet};
use game_net::snapshot::{Command, CommandQueue, ConnectionMessage, DeltaQueue};
use game_net::Socket;
use tokio::runtime::Runtime;

pub use self::conn::ServerConnection;
use self::conn::State;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum NetSet {
    ReadCommands,
    FlushBuffers,
    WriteCommands,
}

/// Client-side network plugin
#[derive(Clone, Debug, Default)]
pub struct NetPlugin {}

impl bevy::prelude::Plugin for NetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let queue = CommandQueue::new();

        let map = EntityMap::default();

        let mut world = WorldState::new();
        world.insert(Instant::now() - Duration::from_millis(50));

        app.insert_resource(queue);
        app.insert_resource(world);
        app.insert_resource(map.clone());
        app.insert_resource(ServerConnection::new(map));
        app.insert_resource(DeltaQueue::new());
        app.insert_resource(Backlog::new());

        app.add_system(flush_command_queue.in_set(NetSet::ReadCommands));

        app.add_system(conn::update_connection_state);
        app.add_system(world::apply_world_delta);
        app.add_system(world::flush_delta_queue);

        app.add_system(interpolate::interpolate_translation);
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
                        id: None,
                        conn: conn.id,
                        command: Command::Disconnected,
                        snapshot: Instant::now(),
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
    queue: Res<CommandQueue>,
    // mut entities: Query<(&mut Transform,)>,
    // hosts: Query<bevy::ecs::entity::Entity, With<HostPlayer>>,
    conn: Res<ServerConnection>,
    mut world: ResMut<WorldState>,
) {
    // Limit the maximum number of iterations in this frame.
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 8192;

    while let Some(msg) = queue.pop() {
        match msg.command {
            Command::Connected => {
                conn.push_state(State::Connected);
                continue;
            }
            Command::Disconnected => {
                conn.push_state(State::Disconnected);
                continue;
            }
            _ => (),
        }

        // Snapshot arrived after we already consumed the frame.
        if let Some(view) = world.back() {
            if msg.snapshot < view.creation() {
                let diff = view.creation() - msg.snapshot;
                tracing::warn!("dropping snapshot; arrived {:?} too late", diff);

                continue;
            }
        }

        if world.get(msg.snapshot).is_none() {
            world.insert(msg.snapshot);
        }

        let mut view = world.get_mut(msg.snapshot).unwrap();

        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                data,
            } => {
                view.spawn(Entity {
                    id,
                    transform: Transform {
                        translation,
                        rotation,
                        scale: Vec3::splat(1.0),
                    },
                    body: data,
                    components: Components::new(),
                });
            }
            Command::EntityDestroy { id } => {
                view.despawn(id);
            }
            Command::EntityTranslate { id, translation } => {
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.translation = translation;
            }
            Command::EntityRotate { id, rotation } => {
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.rotation = rotation;
            }
            Command::EntityVelocity { id, linvel, angvel } => {}
            Command::EntityHealth { id, health } => {
                let mut entity = view.get_mut(id).unwrap();

                if let EntityBody::Actor(actor) = &mut entity.body {
                    actor.health = health;
                } else {
                    tracing::warn!("tried to apply health to a non-actor entity");
                }
            }
            Command::EntityAction { id: _, action: _ } => todo!(),
            Command::SpawnHost { id } => {
                view.spawn_host(id);
                conn.set_host(id);
            }
            Command::Connected => (),
            Command::Disconnected => (),
            Command::ReceivedCommands { ids } => {
                let mut ov = conn.overrides().write();
                for id in ids {
                    ov.remove(id.id);
                }
            }
        }

        iterations += 1;
        if iterations >= MAX_ITERATIONS {
            break;
        }
    }
}
