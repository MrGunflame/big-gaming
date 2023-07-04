mod conn;
pub mod interpolate;
mod prediction;
mod world;

use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::ResMut;
use game_common::components::actions::Actions;
use game_common::components::components::Components;
use game_common::components::items::Item;
use game_common::components::transform::Transform;
use game_common::units::Mass;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::world::WorldState;
use game_net::backlog::Backlog;
use game_net::conn::{Connect, Connection, ConnectionHandle};
use game_net::proto::{Decode, Packet};
use game_net::snapshot::{Command, CommandQueue, ConnectionMessage, Response, Status};
use game_net::Socket;
use glam::Vec3;
use tokio::runtime::Runtime;

use crate::state::GameState;

pub use self::conn::ServerConnection;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum NetSet {
    /// Step control tick
    Tick,
    /// Read incoming server frames
    Read,
    /// Flush frames into world
    FlushBuffers,
    /// Write back inputs
    //Write,
    /// Lerp transform
    Interpolate,
}

impl NetSet {
    pub fn first() -> Self {
        Self::Tick
    }

    pub fn last() -> Self {
        Self::Interpolate
    }
}

/// Client-side network plugin
#[derive(Clone, Debug, Default)]
pub struct NetPlugin {}

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let mut world = WorldState::new();
        // Initial empty world state.
        world.insert(ControlFrame(0));

        app.insert_resource(world);
        app.init_resource::<ServerConnection>();
        app.insert_resource(Backlog::new());

        app.add_system(conn::tick_game.in_set(NetSet::Tick));
        app.add_system(flush_command_queue.in_set(NetSet::Read));
        app.add_system(world::apply_world_delta.in_set(NetSet::FlushBuffers));

        app.add_system(interpolate::interpolate_translation.in_set(NetSet::Interpolate));
        app.add_system(interpolate::interpolate_rotation.in_set(NetSet::Interpolate));

        app.configure_set(NetSet::Interpolate.after(NetSet::FlushBuffers));
        app.configure_set(NetSet::FlushBuffers.after(NetSet::Read));
        app.configure_set(NetSet::Read.after(NetSet::Tick));
    }
}

pub fn spawn_conn(
    queue: CommandQueue,
    addr: SocketAddr,
    control_frame: ControlFrame,
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
                Connection::<Connect>::new(addr, queue.clone(), sock.clone(), control_frame);

            tokio::task::spawn(async move {
                if let Err(err) = (&mut conn).await {
                    tracing::error!("server error: {}", err);
                    queue.push(ConnectionMessage {
                        id: None,
                        conn: conn.id,
                        command: Command::Disconnected,
                        control_frame: ControlFrame(0),
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

fn flush_command_queue(mut conn: ResMut<ServerConnection>, mut world: ResMut<WorldState>) {
    // Limit the maximum number of iterations in this frame.
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 8192;

    // Collect all processed commands to notify the server.
    let mut ids = Vec::new();

    while let Some(msg) = conn.queue.pop() {
        if let Some(id) = msg.id {
            ids.push(Response {
                id,
                status: Status::Received,
            });
        }

        match msg.command {
            Command::Connected => {
                conn.writer.update(GameState::World);
                continue;
            }
            Command::Disconnected => {
                conn.shutdown();
                continue;
            }
            Command::ReceivedCommands { ids } => {
                let view = world.front().unwrap();

                for cmd in ids {
                    conn.overrides.validate_pre_removal(cmd.id, view);
                    conn.overrides.remove(cmd.id);
                }

                continue;
            }
            _ => (),
        }

        // Snapshot arrived after we already consumed the frame.
        // if let Some(view) = world.back() {
        //     if msg.control_frame < view.control_frame() {
        //         let diff = view.control_frame() - msg.control_frame;
        //         tracing::warn!(
        //             "dropping snapshot {:?}; arrived {:?} CFs too late (tail = {:?})",
        //             msg.control_frame,
        //             diff,
        //             view.control_frame(),
        //         );

        //         continue;
        //     }
        // }

        let Some(mut view) = world.get_mut(msg.control_frame) else {
            // If the control frame does not exist on the client ast least one of these issues are to blame:
            // 1. The server is sending garbage data, refereing to a control frame that has either already
            //    passed or is still too far in the future.
            // 2. The client's clock is desynced and creating new snapshots too slow/fast.
            // 3. The server's clock is desynced and creating new snapshots too slow/fast.
            let front = world.front().unwrap();
            let back = world.back().unwrap();
            tracing::warn!("received snapshot for unknwon control frame: {:?} (snapshots  {:?}..={:?} exist)", msg.control_frame, front.control_frame(), back.control_frame());
            continue;
        };

        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                data,
            } => {
                dbg!(id);
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
                conn.host = id;
            }
            Command::InventoryItemAdd { entity, id, item } => {
                let item = Item {
                    id: item,
                    components: Components::default(),
                    mass: Mass::default(),
                    actions: Actions::default(),
                    resistances: None,
                    equipped: false,
                    hidden: false,
                };

                let mut inventories = view.inventories_mut();

                let mut inventory = inventories.get_mut_or_insert(entity);
                // FIXME: Don't unwrap
                inventory.insert(item).unwrap();
            }
            Command::InventoryItemRemove { entity, id } => {
                let mut inventories = view.inventories_mut();

                if let Some(mut inventory) = inventories.get_mut(entity) {
                    inventory.remove(id);
                } else {
                    tracing::warn!(
                        "requested inventory on entity that has no inventory (or does not exist)"
                    );
                }
            }
            Command::InventoryUpdate {
                entity,
                id,
                equipped,
                hidden,
            } => {
                todo!();
            }
            Command::Connected => (),
            Command::Disconnected => (),
            Command::ReceivedCommands { ids: _ } => unreachable!(),
        }

        iterations += 1;
        if iterations >= MAX_ITERATIONS {
            break;
        }
    }

    conn.send(Command::ReceivedCommands { ids });
}
