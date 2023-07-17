mod inventory;

use ahash::HashSet;
use bevy_app::{App, Plugin};
use bevy_ecs::system::{Res, ResMut};
use game_common::components::combat::Health;
use game_common::components::components::Components;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event, EventKind, EventQueue};
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Actor, Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::source::StreamingSource;
use game_common::world::world::{AsView, WorldState};
use game_common::world::CellId;
use game_core::modules::Modules;
use game_net::conn::ConnectionId;
use game_net::snapshot::{
    Command, CommandQueue, ConnectionMessage, EntityCreate, EntityDestroy, EntityRotate,
    EntityTranslate, InventoryItemAdd, Response, SpawnHost, Status,
};
use game_script::events::Events;
use game_script::scripts::Scripts;
use game_script::ScriptServer;
use glam::Vec3;

use crate::config::Config;
use crate::conn::{Connection, Connections};
use crate::entity::ServerEntityGenerator;
use crate::net::state::{Cells, ConnectionState};
use crate::state::State;
use crate::world::level::Level;

pub struct ServerPlugins;

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerEntityGenerator::new());
        app.insert_resource(WorldState::new());

        app.add_system(tick);
    }
}

// All systems need to run sequentially.
pub fn tick(
    conns: Res<Connections>,
    mut world: ResMut<WorldState>,
    queue: Res<CommandQueue>,
    mut level: ResMut<Level>,
    mut pipeline: ResMut<game_physics::Pipeline>,
    mut event_queue: ResMut<EventQueue>,
    server: Res<ScriptServer>,
    mut scripts: ResMut<Scripts>,
    modules: Res<Modules>,
    mut state: ResMut<State>,
) {
    update_client_heads(&conns, &mut world, &mut state);
    flush_command_queue(
        &conns,
        &queue,
        &mut world,
        &mut event_queue,
        &modules,
        &state.config,
    );

    crate::world::level::update_level_cells(&mut world, &mut level, &modules, &mut event_queue);

    game_script::plugin::flush_event_queue(&mut event_queue, &mut world, &server, &scripts);

    #[cfg(feature = "physics")]
    pipeline.step(&mut world, &mut event_queue);

    update_scripts(&world, &mut scripts, &modules);

    // Push snapshots last always
    update_snapshots(&conns, &world);
}

fn update_client_heads(conns: &Connections, world: &mut WorldState, state: &mut State) {
    let control_frame = *state.control_frame.lock();

    world.insert(*state.control_frame.lock());

    for conn in conns.iter() {
        let mut state = conn.state().write();

        // The const client interpolation delay.
        let client_cf = control_frame - state.peer_delay;

        state.client_cf = client_cf;
    }

    if world.len() > 120 {
        world.pop();
    }
}

fn flush_command_queue(
    connections: &Connections,
    queue: &CommandQueue,
    world: &mut WorldState,
    events: &mut EventQueue,
    modules: &Modules,
    config: &Config,
) {
    while let Some(msg) = queue.pop() {
        tracing::trace!("got command {:?}", msg.command);

        let conn = connections.get(msg.conn).unwrap();
        let client_cf = conn.state().read().client_cf;

        // Fetch the world state at the client's computed render time.
        // Note that the client may be too far in the past to go back.
        // In that case we must chose the oldest snapshot.
        let mut view;
        {
            let opt = world.get_mut(client_cf);
            if let Some(v) = opt {
                view = v;
            } else {
                // Note that this `drop` is necessary as `Option<WorldViewMut>` has a `Drop`
                // impl, even thought at this point it never actually needs to drop anything
                // because it is `None`.
                drop(opt);
                match world.front_mut() {
                    Some(v) => view = v,
                    None => {
                        tracing::warn!("no snapshots");
                        return;
                    }
                }
            }
        }

        if let Some(id) = msg.id {
            conn.push_proc_msg(id);
        }

        let mut state = conn.state().write();

        match msg.command {
            Command::EntityCreate(event) => {}
            Command::EntityDestroy(event) => {}
            Command::EntityTranslate(event) => {
                let Some(entity_id) = state.entities.get(event.id) else {
                    continue;
                };

                let mut entity = view.get_mut(entity_id).unwrap();
                entity.transform.translation = event.translation;
            }
            Command::EntityRotate(event) => {
                let Some(entity_id) = state.entities.get(event.id) else {
                    continue;
                };

                let mut entity = view.get_mut(entity_id).unwrap();
                entity.transform.rotation = event.rotation;
            }
            Command::EntityHealth(event) => {
                tracing::warn!("received EntityHealth from client, ignored");
            }
            Command::EntityAction(event) => {
                let Some(entity_id) = state.entities.get(event.id) else {
                    continue;
                };

                events.push(Event::Action(ActionEvent {
                    entity: entity_id,
                    invoker: entity_id,
                    action: event.action,
                }));

                // TODO
            }
            Command::Connected(event) => {
                let id = view.spawn(Entity {
                    id: EntityId::dangling(),
                    transform: Transform::from_translation(Vec3::new(10.0, 32.0, 10.0)),
                    body: EntityBody::Actor(Actor {
                        race: RaceId(1.into()),
                        health: Health::new(50),
                    }),
                    components: Components::new(),
                });

                state.entities.insert(id);

                view.insert_streaming_source(
                    id,
                    StreamingSource {
                        distance: config.player_streaming_source_distance,
                    },
                );

                // At the connection time the delay must be 0, meaning the player is spawned
                // without delay.
                debug_assert_eq!(state.peer_delay, ControlFrame(0));

                state.host.entity = Some(id);
                state.peer_delay = event.peer_delay;
                state.cells = Cells::new(CellId::new(0.0, 0.0, 0.0));

                tracing::info!("spawning host {:?} in cell", msg.id);
            }
            Command::Disconnected => {
                if let Some(id) = state.host.entity {
                    if view.despawn(id).is_none() {
                        tracing::warn!("attempted to destroy an unknown entity {:?}", id);
                    }
                }

                // Remove the player from the connections ref.
                connections.remove(msg.conn);
            }
            Command::SpawnHost(event) => (),
            Command::InventoryItemAdd(event) => {
                // Server-only frame
            }
            Command::InventoryItemRemove(event) => {
                // Server-only frame
            }
            Command::InventoryUpdate(event) => {
                // Server-only frame
            }
            Command::ReceivedCommands(ids) => (),
        }

        drop(view);
    }
}

fn update_scripts(world: &WorldState, scripts: &mut Scripts, modules: &Modules) {
    let Some(view) = world.back() else {
        return;
    };

    for event in view.deltas() {
        match event {
            EntityChange::Create { entity } => {
                // Register events for all components directly on the entity.
                for (id, _) in entity.components.iter() {
                    let module = modules.get(id.module).unwrap();
                    let handles = module.records.get_scripts(id.record).unwrap();

                    for handle in handles {
                        for event in handle.events.iter() {
                            scripts.push(Some(entity.id), event, handle.handle.clone());
                        }
                    }
                }

                // Register for events on inventory items.
                if let Some(inventory) = view.inventories().get(entity.id) {
                    for item in inventory.iter() {
                        let module = modules.get(item.item.id.0.module).unwrap();
                        let handles = module.records.get_scripts(item.item.id.0.record).unwrap();

                        for handle in handles {
                            for event in handle.events.iter() {
                                // FIXME: This should be using InventoryId.
                                scripts.push(Some(entity.id), event, handle.handle.clone());
                            }
                        }

                        // Register for events on item components.
                        for (id, _) in item.item.components.iter() {
                            let module = modules.get(id.module).unwrap();
                            let handles = module.records.get_scripts(id.record).unwrap();

                            for handle in handles {
                                for event in handle.events.iter() {
                                    scripts.push(Some(entity.id), event, handle.handle.clone());
                                }
                            }
                        }

                        // Register for events on item actions.
                        for id in item.item.actions.iter() {
                            let module = modules.get(id.0.module).unwrap();
                            let handles = module.records.get_scripts(id.0.record).unwrap();

                            for handle in handles {
                                // All actions must only expose a action event.
                                debug_assert_eq!(handle.events, Events::ACTION);
                                scripts.push(
                                    Some(entity.id),
                                    EventKind::Action,
                                    handle.handle.clone(),
                                );
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

fn update_snapshots(
    connections: &Connections,
    // FIXME: Make dedicated type for all shared entities.
    // mut entities: Query<(&mut Entity, &Transform)>,
    world: &WorldState,
) {
    for conn in connections.iter() {
        update_client(&conn, world);
    }
}

fn update_client(conn: &Connection, world: &WorldState) {
    let state = &mut *conn.state().write();

    let Some(host_id) = state.host.entity else {
        return;
    };

    // let Some(prev) = world.at(state.head.saturating_sub(1)) else {
    //     return;
    // };

    let Some(curr) = world.back() else {
        return;
    };

    let host = curr.get(host_id).unwrap();
    let cell_id = CellId::from(host.transform.translation);

    let streaming_source = curr.streaming_sources().get(host_id).unwrap();

    // Send full state
    // The delta from the current frame is "included" in the full update.
    if state.full_update {
        state.full_update = false;

        state.entities.clear();
        state.known_entities.clear();

        tracing::info!(
            "sending full update to host in cell {:?} for cells: {:?}",
            cell_id,
            state.cells.cells(),
        );

        for id in state.cells.cells() {
            let cell = curr.cell(*id);

            for entity in cell.iter() {
                state.known_entities.insert(entity.clone());

                let entity_id = state.entities.insert(entity.id);

                conn.handle().send_cmd(ConnectionMessage {
                    id: None,
                    conn: ConnectionId(0),
                    control_frame: curr.control_frame(),
                    command: Command::EntityCreate(EntityCreate {
                        id: entity_id,
                        translation: entity.transform.translation,
                        rotation: entity.transform.rotation,
                        data: entity.body.clone(),
                    }),
                });

                // Sync the entity inventory, if it has one.
                if let Some(inventory) = curr.inventories().get(entity.id) {
                    for item in inventory.iter() {
                        conn.handle().send_cmd(ConnectionMessage {
                            id: None,
                            conn: ConnectionId(0),
                            control_frame: curr.control_frame(),
                            command: Command::InventoryItemAdd(InventoryItemAdd {
                                entity: entity_id,
                                slot: item.id,
                                item: item.item.id,
                            }),
                        });
                    }
                }
            }
        }

        // Also sent the host.
        let id = state.entities.get(host_id).unwrap();
        conn.handle().send_cmd(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: curr.control_frame(),
            command: Command::SpawnHost(SpawnHost { id }),
        });

        return;
    }

    // `Cells::set` may allocate so avoid calling it unless
    // necessary.
    if state.cells.origin() != cell_id {
        tracing::info!("Moving host from {:?} to {:?}", state.cells, cell_id);

        state.cells.set(cell_id, streaming_source.distance);
    }

    let events = update_player_cells(curr, state);

    // The host should never be destroyed.
    if cfg!(debug_assertions) {
        for event in &events {
            match event {
                EntityChange::Destroy { id } => {
                    assert_ne!(*id, host.id);
                }
                _ => (),
            }
        }
    }

    for cmd in update_client_entities(state, events) {
        conn.handle().send_cmd(ConnectionMessage {
            id: None,
            conn: ConnectionId(0),
            control_frame: curr.control_frame(),
            command: cmd,
        });
    }

    // Acknowledge client commands.
    let ids = conn.take_proc_msg();
    if !ids.is_empty() {
        conn.handle().send_cmd(ConnectionMessage {
            id: None,
            conn: conn.id(),
            control_frame: ControlFrame(0),
            command: Command::ReceivedCommands(
                ids.into_iter()
                    .map(|id| Response {
                        id,
                        status: Status::Received,
                    })
                    .collect(),
            ),
        });
    }
}

/// Update a player that hasn't moved cells.
fn update_player_cells<V>(view: V, state: &ConnectionState) -> Vec<EntityChange>
where
    V: AsView,
{
    let mut events = Vec::new();

    let mut stale_entities: HashSet<_> = state.known_entities.entities.keys().copied().collect();

    for id in state.cells.iter() {
        let cell = view.cell(id);

        for entity in cell.iter() {
            if !state.known_entities.contains(entity.id) {
                events.push(EntityChange::Create {
                    entity: entity.clone(),
                });

                continue;
            }

            stale_entities.remove(&entity.id);

            let known = state.known_entities.get(entity.id).unwrap();

            if known.transform.translation != entity.transform.translation {
                events.push(EntityChange::Translate {
                    id: entity.id,
                    translation: entity.transform.translation,
                });
            }

            if known.transform.rotation != entity.transform.rotation {
                events.push(EntityChange::Rotate {
                    id: entity.id,
                    rotation: entity.transform.rotation,
                });
            }
        }
    }

    // Despawn all entities that were not existent in any of the player's cells.
    for id in stale_entities {
        events.push(EntityChange::Destroy { id });
    }

    events
}

fn update_client_entities(state: &mut ConnectionState, events: Vec<EntityChange>) -> Vec<Command> {
    let mut cmds = Vec::new();

    for event in events {
        let cmd = match event {
            EntityChange::Create { entity } => {
                let entity_id = state.entities.insert(entity.id);
                state.known_entities.insert(entity.clone());

                Command::EntityCreate(EntityCreate {
                    id: entity_id,
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    data: entity.body,
                })
            }
            EntityChange::Destroy { id } => {
                let entity_id = state.entities.remove(id).unwrap();
                state.known_entities.remove(id);

                Command::EntityDestroy(EntityDestroy { id: entity_id })
            }
            EntityChange::Translate { id, translation } => {
                let entity_id = state.entities.get(id).unwrap();
                let entity = state.known_entities.get_mut(id).unwrap();

                entity.transform.translation = translation;

                Command::EntityTranslate(EntityTranslate {
                    id: entity_id,
                    translation,
                })
            }
            EntityChange::Rotate { id, rotation } => {
                let entity_id = state.entities.get(id).unwrap();
                let entity = state.known_entities.get_mut(id).unwrap();

                entity.transform.rotation = rotation;

                Command::EntityRotate(EntityRotate {
                    id: entity_id,
                    rotation,
                })
            }
            _ => todo!(),
        };

        cmds.push(cmd);
    }

    cmds
}

#[cfg(test)]
mod tests {
    use game_common::components::object::ObjectId;
    use game_common::components::transform::Transform;
    use game_common::entity::EntityId;
    use game_common::record::RecordReference;
    use game_common::world::control_frame::ControlFrame;
    use game_common::world::entity::{Entity, EntityBody, Object};
    use game_common::world::snapshot::EntityChange;
    use game_common::world::world::WorldState;
    use game_common::world::CellId;
    use glam::{IVec3, Vec3};

    use crate::net::state::ConnectionState;
    use crate::plugins::update_client_entities;

    use super::update_player_cells;

    fn create_test_entity() -> Entity {
        Entity {
            id: EntityId::dangling(),
            transform: Transform::default(),
            body: EntityBody::Object(Object {
                id: ObjectId(RecordReference::STUB),
            }),
            components: Default::default(),
        }
    }

    #[test]
    fn player_update_cells_spawn_entity() {
        let mut world = WorldState::new();
        let cf = ControlFrame(0);
        world.insert(cf);

        let mut view = world.get_mut(cf).unwrap();
        view.spawn(create_test_entity());

        let mut state = ConnectionState::new();
        let events = update_player_cells(&view, &state);
        update_client_entities(&mut state, events.clone());

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EntityChange::Create { entity: _ }));
    }

    #[test]
    fn player_update_cells_translate_entity() {
        let mut world = WorldState::new();
        let cf = ControlFrame(0);
        world.insert(cf);

        let mut view = world.get_mut(cf).unwrap();
        let entity_id = view.spawn(create_test_entity());

        let mut state = ConnectionState::new();
        let events = update_player_cells(&view, &state);
        update_client_entities(&mut state, events);

        let mut entity = view.get_mut(entity_id).unwrap();
        entity.transform.translation = Vec3::splat(1.0);
        drop(entity);

        let events = update_player_cells(&view, &mut state);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            EntityChange::Translate {
                id: _,
                translation: _,
            }
        ));
    }

    #[test]
    fn player_upate_cells_despawn_entity() {
        let mut world = WorldState::new();
        let cf = ControlFrame(0);
        world.insert(cf);

        let mut view = world.get_mut(cf).unwrap();
        let entity_id = view.spawn(create_test_entity());

        let mut state = ConnectionState::new();
        let events = update_player_cells(&view, &state);
        update_client_entities(&mut state, events);

        view.despawn(entity_id);

        let events = update_player_cells(&view, &mut state);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EntityChange::Destroy { id: _ }));
    }

    #[test]
    fn player_update_cells_entity_leave_cells() {
        let mut world = WorldState::new();
        let cf = ControlFrame(0);
        world.insert(cf);

        let mut view = world.get_mut(cf).unwrap();
        let entity_id = view.spawn(create_test_entity());

        let mut state = ConnectionState::new();
        let events = update_player_cells(&view, &state);
        update_client_entities(&mut state, events);

        let mut entity = view.get_mut(entity_id).unwrap();
        entity.transform.translation = Vec3::splat(1024.0);
        drop(entity);

        let events = update_player_cells(&view, &mut state);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EntityChange::Destroy { id: _ }));
    }

    #[test]
    fn player_update_cells_entity_translate_parallel() {
        let distance = 0;

        let mut world = WorldState::new();
        let cf = ControlFrame(0);
        world.insert(cf);

        let mut view = world.get_mut(cf).unwrap();
        let entity_id = view.spawn(create_test_entity());

        let mut state = ConnectionState::new();
        state.cells.set(CellId::from_i32(IVec3::new(0, 0, 0)), 0);
        let events = update_player_cells(&view, &mut state);
        update_client_entities(&mut state, events);

        let new_cell = CellId::from_i32(IVec3::splat(1));
        state.cells.set(new_cell, distance);

        let mut entity = view.get_mut(entity_id).unwrap();
        entity.transform.translation = new_cell.min();
        drop(entity);

        let events = update_player_cells(&view, &mut state);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            EntityChange::Translate {
                id: _,
                translation: _,
            }
        ));
    }
}
