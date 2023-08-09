mod conn;
mod entities;
pub mod interpolate;
mod prediction;
mod socket;
mod world;

use std::time::Instant;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::dbg;
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::{Res, ResMut};
use game_common::components::actions::Actions;
use game_common::components::components::Components;
use game_common::components::items::Item;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::units::Mass;
use game_common::world::entity::{Entity, EntityBody};
use game_core::counter::Interval;
use game_core::time::Time;
use game_net::snapshot::{Command, Response, Status};
use glam::Vec3;

use crate::state::GameState;

pub use self::conn::ServerConnection;
use self::world::CommandBuffer;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum NetSet {
    /// Step control tick
    Tick,
    WriteBack,
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
        app.init_resource::<ServerConnection<Interval>>();
        app.insert_resource(CommandBuffer::new());

        app.add_system(tick.in_set(NetSet::Tick));
        app.add_system(world::write_back.in_set(NetSet::WriteBack));

        app.add_system(interpolate::interpolate_translation.in_set(NetSet::Interpolate));
        app.add_system(interpolate::interpolate_rotation.in_set(NetSet::Interpolate));

        app.configure_set(NetSet::WriteBack.after(NetSet::Tick));
        app.configure_set(NetSet::Interpolate.after(NetSet::WriteBack));
    }
}

pub fn tick(
    mut conn: ResMut<ServerConnection<Interval>>,
    mut buffer: ResMut<CommandBuffer>,
    time: Res<Time>,
) {
    conn::tick_game(&time, &mut conn);
    flush_command_queue(&mut conn);
    world::apply_world_delta(&mut conn, &mut buffer);
}

fn flush_command_queue<I>(conn: &mut ServerConnection<I>) {
    // Limit the maximum number of iterations in this frame.
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 8192;

    // Collect all processed commands to notify the server.
    let mut ids = Vec::new();

    while let Some(msg) = conn.queue.pop() {
        dbg!(&msg);
        if let Some(id) = msg.id {
            ids.push(Response {
                id,
                status: Status::Received,
            });
        }

        match msg.command {
            Command::Connected(_) => {
                conn.writer.update(GameState::World);
                continue;
            }
            Command::Disconnected => {
                conn.shutdown();
                continue;
            }
            Command::ReceivedCommands(ids) => {
                conn.metrics.commands_acks += ids.len() as u64;

                // Note that we can't remove the prediction until we've reached
                // the CF that the server acknowledged that it recevied our
                // commands.
                conn.commands_in_frame
                    .entry(msg.control_frame)
                    .or_default()
                    .extend(ids.into_iter().map(|res| res.id));

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

        let Some(mut view) = conn.world.get_mut(msg.control_frame) else {
            // If the control frame does not exist on the client ast least one of these issues are to blame:
            // 1. The server is sending garbage data, refereing to a control frame that has either already
            //    passed or is still too far in the future.
            // 2. The client's clock is desynced and creating new snapshots too slow/fast.
            // 3. The server's clock is desynced and creating new snapshots too slow/fast.
            let front = conn.world.front().unwrap();
            let back = conn.world.back().unwrap();
            tracing::warn!(
                "received snapshot for unknwon control frame: {:?} (snapshots  {:?}..={:?} exist)",
                msg.control_frame,
                front.control_frame(),
                back.control_frame()
            );
            continue;
        };

        match msg.command {
            Command::EntityCreate(event) => {
                let id = view.spawn(Entity {
                    id: EntityId::dangling(),
                    transform: Transform {
                        translation: event.translation,
                        rotation: event.rotation,
                        scale: Vec3::splat(1.0),
                    },
                    body: event.data,
                    components: Components::new(),
                    is_host: false,
                });

                conn.server_entities.insert(id, event.id);
            }
            Command::EntityDestroy(event) => match conn.server_entities.remove(event.id) {
                Some(id) => {
                    if view.despawn(id).is_none() {
                        tracing::warn!("attempted to destroy a non-existant entity {:?}", id);
                    }
                }
                None => (),
            },
            Command::EntityTranslate(event) => match conn.server_entities.get(event.id) {
                Some(id) => match view.get_mut(id) {
                    Some(mut entity) => {
                        entity.set_translation(event.translation);
                    }
                    None => {
                        tracing::warn!("received translation for unknown entity {:?}", id);
                    }
                },
                None => (),
            },
            Command::EntityRotate(event) => match conn.server_entities.get(event.id) {
                Some(id) => match view.get_mut(id) {
                    Some(mut entity) => entity.set_rotation(event.rotation),
                    None => {
                        tracing::warn!("received rotation for unknown entity {:?}", id);
                    }
                },
                None => (),
            },
            Command::EntityHealth(event) => match conn.server_entities.get(event.id) {
                Some(id) => {
                    let mut entity = view.get_mut(id).unwrap();

                    todo!();

                    if let EntityBody::Actor(actor) = &mut entity.body {
                        actor.health = event.health;
                    } else {
                        tracing::warn!("tried to apply health to a non-actor entity");
                    }
                }
                None => (),
            },
            Command::EntityAction(event) => todo!(),
            Command::SpawnHost(event) => match conn.server_entities.get(event.id) {
                Some(id) => {
                    view.spawn_host(id);
                    conn.host = id;
                }
                None => (),
            },
            Command::InventoryItemAdd(event) => {
                match conn.server_entities.get(event.entity) {
                    Some(id) => {
                        let item = Item {
                            id: event.item,
                            components: Components::default(),
                            mass: Mass::default(),
                            actions: Actions::default(),
                            resistances: None,
                            equipped: false,
                            hidden: false,
                        };

                        let mut inventories = view.inventories_mut();

                        let mut inventory = inventories.get_mut_or_insert(id);
                        // FIXME: Don't unwrap
                        inventory.insert(item).unwrap();
                    }
                    None => (),
                }
            }
            Command::InventoryItemRemove(event) => match conn.server_entities.get(event.entity) {
                Some(id) => {
                    let mut inventories = view.inventories_mut();

                    if let Some(mut inventory) = inventories.get_mut(id) {
                        inventory.remove(event.slot);
                    } else {
                        tracing::warn!(
                                "requested inventory on entity that has no inventory (or does not exist)"
                            );
                    }
                }
                None => (),
            },
            Command::InventoryUpdate(event) => {
                todo!();
            }
            Command::PlayerMove(_) => {
                // Client-only frame.
                tracing::warn!("received client-only `PlayerMove` frame from server");
            }
            Command::Connected(_) => (),
            Command::Disconnected => (),
            Command::ReceivedCommands(_) => unreachable!(),
        }

        iterations += 1;
        if iterations >= MAX_ITERATIONS {
            break;
        }
    }

    conn.send(Command::ReceivedCommands(ids));
}

#[cfg(test)]
mod tests {
    use game_common::assert_approx_eq;
    use game_common::components::items::ItemId;
    use game_common::entity::EntityId;
    use game_common::net::ServerEntity;
    use game_common::record::RecordReference;
    use game_common::world::control_frame::ControlFrame;
    use game_common::world::entity::{Entity, EntityBody, Item};
    use game_core::counter::ManualInterval;
    use game_core::time::Time;
    use game_net::conn::ConnectionId;
    use game_net::proto::MoveBits;
    use game_net::snapshot::{
        Command, CommandId, ConnectionMessage, EntityCreate, EntityTranslate, PlayerMove, Response,
        Status,
    };
    use glam::{Quat, Vec3};

    use crate::config::{Config, Network};
    use crate::net::conn::tick_game;
    use crate::net::tick;
    use crate::state::GameStateWriter;

    use super::world::{apply_world_delta, CommandBuffer};
    use super::{flush_command_queue, ServerConnection};

    fn create_test_entity() -> EntityCreate {
        EntityCreate {
            id: ServerEntity(0),
            translation: Vec3::splat(0.0),
            rotation: Quat::IDENTITY,
            data: EntityBody::Item(Item {
                id: ItemId(RecordReference::STUB),
            }),
        }
    }

    fn create_test_conn(delay: u16) -> ServerConnection<ManualInterval> {
        let config = Config {
            timestep: 60,
            network: Network {
                interpolation_frames: delay,
                prediction: true,
            },
        };
        ServerConnection::new_with_interval(GameStateWriter::noop(), &config, ManualInterval::new())
    }

    #[test]
    fn flush_command_queue_no_delay() {
        let delay = 6;
        let mut conn = create_test_conn(delay);

        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: ControlFrame(0),
            command: Command::EntityCreate(create_test_entity()),
        });

        flush_command_queue(&mut conn);

        let view = conn.world.get(ControlFrame(0)).unwrap();
        assert_eq!(view.len(), 1);
    }

    #[test]
    fn apply_world_delta_interpolation_delay() {
        let delay = 6;

        // Note that time is irrelevant because we drive the interval
        // ourselves with `ManualInterval`.
        let time = Time::new();

        let mut conn = create_test_conn(delay);

        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: ControlFrame(0),
            command: Command::EntityCreate(create_test_entity()),
        });

        flush_command_queue(&mut conn);

        for _ in 0..delay {
            let mut buffer = CommandBuffer::new();
            apply_world_delta(&mut conn, &mut buffer);
            assert_eq!(buffer.len(), 0);

            conn.game_tick.interval.set_ready();
            tick_game(&time, &mut conn);
        }

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn predict_translation() {
        let delay = 6;

        let time = Time::new();

        let mut conn = create_test_conn(delay);

        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: ControlFrame(0),
            command: Command::EntityCreate(create_test_entity()),
        });

        // Create initial world state.
        for _ in 0..delay + 1 {
            flush_command_queue(&mut conn);

            let mut buffer = CommandBuffer::new();
            apply_world_delta(&mut conn, &mut buffer);

            conn.game_tick.interval.set_ready();
            tick_game(&time, &mut conn);
        }

        let cmd_id = CommandId(1);
        let cmd = Command::PlayerMove(PlayerMove {
            entity: ServerEntity(0),
            bits: MoveBits {
                forward: true,
                back: false,
                left: false,
                right: false,
            },
        });

        let entity = conn.world.at(0).unwrap().iter().next().unwrap().clone();
        assert_eq!(entity.transform.translation, Vec3::splat(0.0));

        conn.predictions.push(entity.id, cmd_id, cmd);

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 1);

        match buffer.pop().unwrap() {
            super::world::Command::Translate {
                entity: _,
                start: _,
                end: _,
                dst,
            } => {
                // Should be `Vec3(0.0, 0.0, -1.0)`, but FP memes are
                // happening, close enough.
                assert_eq!(dst, Vec3::new(-1.7484555e-7, 0.0, -1.0));
            }
            _ => panic!("invalid command, expected `Translate`"),
        }
    }

    #[test]
    fn predict_translation_reconciliation() {
        let delay = 6;

        let time = Time::new();
        let mut conn = create_test_conn(delay);

        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: ControlFrame(0),
            command: Command::EntityCreate(create_test_entity()),
        });

        for _ in 0..delay + 1 {
            flush_command_queue(&mut conn);

            let mut buffer = CommandBuffer::new();
            apply_world_delta(&mut conn, &mut buffer);

            conn.game_tick.interval.set_ready();
            tick_game(&time, &mut conn);
        }

        let cmd_id = CommandId(1);
        let cmd = Command::PlayerMove(PlayerMove {
            entity: ServerEntity(0),
            bits: MoveBits {
                forward: true,
                back: false,
                left: false,
                right: false,
            },
        });

        let entity = conn.world.at(0).unwrap().iter().next().unwrap().clone();
        assert_eq!(entity.transform.translation, Vec3::splat(0.0));

        conn.predictions.push(entity.id, cmd_id, cmd);

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 1);

        match buffer.pop().unwrap() {
            super::world::Command::Translate {
                entity: _,
                start: _,
                end: _,
                dst,
            } => {
                assert_eq!(dst, Vec3::new(-1.7484555e-7, 0.0, -1.0));
            }
            _ => panic!("invalid command, expected `Translate`"),
        }

        conn.game_tick.interval.set_ready();
        tick_game(&time, &mut conn);

        let head = conn.control_frame().head;
        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: head,
            command: Command::ReceivedCommands(vec![Response {
                id: cmd_id,
                status: Status::Received,
            }]),
        });

        flush_command_queue(&mut conn);
        dbg!(&conn.commands_in_frame);
        while conn.last_render_frame.unwrap() < head - 1 {
            let mut buffer = CommandBuffer::new();
            apply_world_delta(&mut conn, &mut buffer);

            assert_eq!(conn.predictions.len(entity.id).unwrap(), 1);

            assert_eq!(buffer.len(), 1);
            match buffer.pop().unwrap() {
                super::world::Command::Translate {
                    entity: _,
                    start: _,
                    end: _,
                    dst,
                } => {
                    assert_eq!(dst, Vec3::new(-1.7484555e-7, 0.0, -1.0));
                }
                _ => panic!("invalid command, expected `Translate`"),
            }

            conn.game_tick.interval.set_ready();
            tick_game(&time, &mut conn);
        }

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 0);

        assert_eq!(conn.predictions.len(entity.id).unwrap(), 0);

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn predict_translation_reconciliation_overwritten() {
        let delay = 6;

        let time = Time::new();
        let mut conn = create_test_conn(delay);

        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: ControlFrame(0),
            command: Command::EntityCreate(create_test_entity()),
        });

        for _ in 0..delay + 1 {
            flush_command_queue(&mut conn);

            let mut buffer = CommandBuffer::new();
            apply_world_delta(&mut conn, &mut buffer);

            conn.game_tick.interval.set_ready();
            tick_game(&time, &mut conn);
        }

        let cmd_id = CommandId(1);
        let cmd = Command::PlayerMove(PlayerMove {
            entity: ServerEntity(0),
            bits: MoveBits {
                forward: true,
                back: false,
                left: false,
                right: false,
            },
        });

        let entity = conn.world.at(0).unwrap().iter().next().unwrap().clone();
        assert_eq!(entity.transform.translation, Vec3::splat(0.0));

        conn.predictions.push(entity.id, cmd_id, cmd);

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 1);

        match buffer.pop().unwrap() {
            super::world::Command::Translate {
                entity: _,
                start: _,
                end: _,
                dst,
            } => {
                assert_eq!(dst, Vec3::new(-1.7484555e-7, 0.0, -1.0));
            }
            _ => panic!("invalid command, expected `Translate`"),
        }

        conn.game_tick.interval.set_ready();
        tick_game(&time, &mut conn);

        // Achknowledged, but overwriten at same time.
        let cmd_id2 = CommandId(2);
        let cmd = Command::PlayerMove(PlayerMove {
            entity: ServerEntity(0),
            bits: MoveBits {
                forward: true,
                back: false,
                left: false,
                right: false,
            },
        });
        conn.predictions.push(entity.id, cmd_id2, cmd);

        let head = conn.control_frame().head;
        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: head,
            command: Command::EntityTranslate(EntityTranslate {
                id: ServerEntity(0),
                translation: Vec3::new(-1.7484555e-7, 0.0, -1.0),
            }),
        });
        conn.queue.push(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: head,
            command: Command::ReceivedCommands(vec![Response {
                id: cmd_id,
                status: Status::Received,
            }]),
        });

        flush_command_queue(&mut conn);
        assert_eq!(conn.predictions.len(entity.id).unwrap(), 2);

        // Wait for render to catch up to head, then the first
        // prediction should be reconciled.
        while conn.last_render_frame.unwrap() < head - 1 {
            let mut buffer = CommandBuffer::new();
            apply_world_delta(&mut conn, &mut buffer);

            assert_eq!(conn.predictions.len(entity.id).unwrap(), 2);

            // FIXME: There isn't anything happening here, but `apply_world_delta`
            // just repeats the predicted translation, which doesn't actually change.
            // In the future this should not return any command if the value doesn't
            // change.
            assert_eq!(buffer.len(), 1);
            match buffer.pop().unwrap() {
                super::world::Command::Translate {
                    entity: _,
                    start: _,
                    end: _,
                    dst,
                } => {
                    assert_eq!(dst, Vec3::new(-3.496911e-7, 0.0, -2.0));
                }
                _ => panic!("invalid command, expected `Translate`"),
            }

            conn.game_tick.interval.set_ready();
            tick_game(&time, &mut conn);
        }

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 1);

        assert_eq!(conn.predictions.len(entity.id).unwrap(), 1);

        conn.game_tick.interval.set_ready();
        tick_game(&time, &mut conn);

        let mut buffer = CommandBuffer::new();
        apply_world_delta(&mut conn, &mut buffer);
        assert_eq!(buffer.len(), 1);

        match buffer.pop().unwrap() {
            super::world::Command::Translate {
                entity: _,
                start: _,
                end: _,
                dst,
            } => {
                assert_eq!(dst, Vec3::new(-3.496911e-7, 0.0, -2.0));
            }
            _ => panic!("invalid command, expected `Translate`"),
        }
    }
}
