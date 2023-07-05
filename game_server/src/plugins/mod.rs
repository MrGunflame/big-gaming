mod inventory;

use ahash::HashSet;
use bevy_app::{App, Plugin};
use bevy_ecs::system::{Commands, Res, ResMut};
use bevy_hierarchy::DespawnRecursiveExt;
use game_common::bundles::ActorBundle;
use game_common::components::combat::Health;
use game_common::components::components::Components;
use game_common::components::player::Player;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_common::entity::{EntityId, EntityMap};
use game_common::events::{ActionEvent, EntityEvent, Event, EventKind, EventQueue};
use game_common::world::control_frame::ControlFrame;
use game_common::world::delta_queue::DeltaQueue;
use game_common::world::entity::{Actor, Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::source::{StreamingSource, StreamingState};
use game_common::world::world::{AsView, WorldState, WorldViewRef};
use game_common::world::CellId;
use game_core::modules::Modules;
use game_net::conn::ConnectionId;
use game_net::snapshot::{Command, CommandQueue, ConnectionMessage, Response, Status};
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
        app.insert_resource(EntityMap::default());

        app.add_system(tick);
    }
}

// All systems need to run sequentially.
pub fn tick(
    commands: Commands,
    conns: Res<Connections>,
    mut world: ResMut<WorldState>,
    queue: Res<CommandQueue>,
    map: Res<EntityMap>,
    mut level: ResMut<Level>,
    mut pipeline: ResMut<game_physics::Pipeline>,
    mut event_queue: ResMut<EventQueue>,
    server: Res<ScriptServer>,
    mut scripts: ResMut<Scripts>,
    modules: Res<Modules>,
    mut state: ResMut<State>,
) {
    let mut delta_queue = DeltaQueue::default();

    update_client_heads(&conns, &mut world, &mut state);
    flush_command_queue(
        commands,
        &conns,
        &queue,
        &map,
        &mut world,
        &mut event_queue,
        &modules,
        &mut delta_queue,
        &state.config,
    );

    crate::world::level::update_level_cells(&mut world, &mut level);

    game_script::plugin::flush_event_queue(&mut event_queue, &mut world, &server, &scripts);

    #[cfg(feature = "physics")]
    pipeline.step(&mut world, &mut event_queue);

    update_scripts(&world, &mut scripts, &modules);

    // Push snapshots last always
    update_snapshots(&conns, &world, &delta_queue);
}

fn update_client_heads(conns: &Connections, world: &mut WorldState, state: &mut State) {
    let control_frame = *state.control_frame.lock();

    world.insert(*state.control_frame.lock());

    for conn in conns.iter() {
        let old_head = conn.state().write().head;

        //let client_time = Instant::now() - Duration::from_millis(100);
        let client_time = control_frame - 5;
        let head = world.index(client_time).unwrap_or(world.len() - 1);

        // assert_ne!(old_head, head);

        conn.state().write().head = head;
    }

    if world.len() > 120 {
        world.pop();
    }
}

fn flush_command_queue(
    mut commands: Commands,
    connections: &Connections,
    queue: &CommandQueue,
    map: &EntityMap,
    world: &mut WorldState,
    events: &mut EventQueue,
    modules: &Modules,
    delta_queue: &mut DeltaQueue,
    config: &Config,
) {
    while let Some(msg) = queue.pop() {
        tracing::trace!("got command {:?}", msg.command);

        let conn = connections.get(msg.conn).unwrap();
        let head = conn.state().read().head;

        // Get the world state at the time the client sent the command.
        // let Some(mut view) = world.at_mut(head) else {
        let Some(mut view) = world.back_mut() else {
            tracing::warn!("No snapshots yet");
            return;
        };

        if let Some(id) = msg.id {
            conn.push_proc_msg(id);
        }

        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                data,
            } => {}
            Command::EntityDestroy { id } => {
                // commands.entity(id).despawn();
            }
            Command::EntityTranslate { id, translation } => {
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.translation = translation;

                let cell = CellId::from(translation);
                delta_queue.push(
                    cell,
                    EntityChange::Translate {
                        id,
                        translation,
                        cell: None,
                    },
                );
            }
            Command::EntityRotate { id, rotation } => {
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.rotation = rotation;

                let cell = CellId::from(entity.transform.translation);
                delta_queue.push(cell, EntityChange::Rotate { id, rotation });
            }
            Command::EntityVelocity { id, linvel, angvel } => {
                let ent = map.get(id).unwrap();

                // let (ent, _, mut velocity) = entities.get_mut(ent).unwrap();
                // velocity.linvel = linvel;
                // velocity.angvel = angvel;
            }
            Command::EntityHealth { id: _, health: _ } => {
                tracing::warn!("received EntityHealth from client, ignored");
            }
            Command::EntityAction { id, action } => {
                events.push(EntityEvent {
                    entity: id,
                    event: Event::Action(ActionEvent {
                        entity: id,
                        invoker: id,
                        action,
                    }),
                });

                // TODO
            }
            Command::Connected => {
                let id = view.spawn(Entity {
                    id: EntityId::dangling(),
                    transform: Transform::from_translation(Vec3::new(10.0, 32.0, 10.0)),
                    body: EntityBody::Actor(Actor {
                        race: RaceId(1.into()),
                        health: Health::new(50),
                    }),
                    components: Components::new(),
                });

                view.upate_streaming_source(
                    id,
                    StreamingSource {
                        state: StreamingState::Create,
                        distance: config.player_streaming_source_distance,
                    },
                );

                let mut actor = ActorBundle::default();
                actor.transform.transform.translation.y += 5.0;
                actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

                let mut cmds = commands.spawn(actor);
                cmds.insert(Player)
                    .insert(StreamingSource::default())
                    .insert(Entity {
                        id,
                        transform: Transform::default(),
                        body: EntityBody::Actor(Actor {
                            race: RaceId(1.into()),
                            health: Health::new(50),
                        }),
                        components: Components::new(),
                    });
                // Human::default().spawn(&assets, &mut cmds);

                let ent = cmds.id();

                // connections
                //     .get_mut(msg.id)
                //     .unwrap()
                //     .data
                //     .handle
                //     .send_cmd(Command::EntityCreate {
                //         id,
                //         kind: EntityKind::Actor(()),
                //         translation: Vec3::new(0.0, 1000.0, 0.0),
                //         rotation: Quat::default(),
                //     });

                // let inventory = Inventory::new();
                // view.inventories_mut().insert(id, inventory);

                // let mut invs = view.inventories_mut();
                // let mut inv = invs.get_mut_or_insert(id);

                // inventory::add_item(
                //     &mut inv,
                //     ItemId(RecordReference {
                //         module: "e9aa65d7953b4132beed9bbcff89e00a".parse().unwrap(),
                //         record: RecordId(3),
                //     }),
                //     &modules,
                // );

                map.insert(id, ent);
                // FIXME: This should not be set in this snapshot, but in the most
                // recent one.
                conn.set_host(id, view.control_frame());

                let mut state = conn.state().write();
                state.id = Some(id);
                state.cells = Cells::new(CellId::new(0.0, 0.0, 0.0));

                tracing::info!("spawning host {:?} in cell", msg.id);
            }
            Command::Disconnected => {
                if let Some(id) = conn.host() {
                    view.despawn(id);
                    let entity = map.get(id).unwrap();
                    commands.entity(entity).despawn_recursive();
                }

                // Remove the player from the connections ref.
                connections.remove(msg.conn);
            }
            Command::SpawnHost { id } => (),
            Command::InventoryItemAdd {
                entity: _,
                id: _,
                item: _,
            } => {
                // Server-only frame
            }
            Command::InventoryItemRemove { entity: _, id: _ } => {
                // Server-only frame
            }
            Command::InventoryUpdate {
                entity: _,
                id: _,
                equipped: _,
                hidden: _,
            } => {
                // Server-only frame
            }
            Command::ReceivedCommands { ids: _ } => (),
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
                            scripts.push(entity.id, event, handle.handle.clone());
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
                                scripts.push(entity.id, event, handle.handle.clone());
                            }
                        }

                        // Register for events on item components.
                        for (id, _) in item.item.components.iter() {
                            let module = modules.get(id.module).unwrap();
                            let handles = module.records.get_scripts(id.record).unwrap();

                            for handle in handles {
                                for event in handle.events.iter() {
                                    scripts.push(entity.id, event, handle.handle.clone());
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
                                scripts.push(entity.id, EventKind::Action, handle.handle.clone());
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
    delta_queue: &DeltaQueue,
) {
    for conn in connections.iter() {
        update_client(&conn, world, &delta_queue);
    }
}

fn update_client(conn: &Connection, world: &WorldState, delta_queue: &DeltaQueue) {
    let state = &mut *conn.state().write();

    let Some(id) = state.id else {
        return;
    };

    // let Some(prev) = world.at(state.head.saturating_sub(1)) else {
    //     return;
    // };

    let Some(curr) = world.back() else {
        return;
    };

    let host = curr.get(id).unwrap();
    let cell_id = CellId::from(host.transform.translation);

    let streaming_source = curr.streaming_sources().get(id).unwrap();

    // Send full state
    // The delta from the current frame is "included" in the full update.
    if state.full_update {
        state.full_update = false;

        tracing::info!(
            "sending full update to host in cell {:?} for cells: {:?}",
            cell_id,
            state.cells.cells(),
        );

        for id in state.cells.cells() {
            let cell = curr.cell(*id);

            for entity in cell.iter() {
                state.known_entities.insert(entity.clone());

                conn.handle().send_cmd(ConnectionMessage {
                    id: None,
                    conn: ConnectionId(0),
                    control_frame: curr.control_frame(),
                    command: Command::EntityCreate {
                        id: entity.id,
                        translation: entity.transform.translation,
                        rotation: entity.transform.rotation,
                        data: entity.body.clone(),
                    },
                });

                // Sync the entity inventory, if it has one.
                if let Some(inventory) = curr.inventories().get(entity.id) {
                    for item in inventory.iter() {
                        conn.handle().send_cmd(ConnectionMessage {
                            id: None,
                            conn: ConnectionId(0),
                            control_frame: curr.control_frame(),
                            command: Command::InventoryItemAdd {
                                entity: entity.id,
                                id: item.id,
                                item: item.item.id,
                            },
                        });
                    }
                }
            }
        }

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

    conn.push(events, curr.control_frame());

    // Acknowledge client commands.
    let ids = conn.take_proc_msg();
    if !ids.is_empty() {
        conn.handle().send_cmd(ConnectionMessage {
            id: None,
            conn: conn.id(),
            control_frame: ControlFrame(0),
            command: Command::ReceivedCommands {
                ids: ids
                    .into_iter()
                    .map(|id| Response {
                        id,
                        status: Status::Received,
                    })
                    .collect(),
            },
        });
    }
}

fn player_move_cells(
    view: WorldViewRef<'_>,
    state: &mut ConnectionState,
    host: &Entity,
    streaming_source: &StreamingSource,
) -> Vec<EntityChange> {
    let mut events = Vec::new();

    let update = state
        .cells
        .set(host.transform.translation.into(), streaming_source.distance);

    for id in update.unloaded() {
        let cell = view.cell(id);

        for entity in cell.iter() {
            debug_assert_ne!(entity.id, host.id);

            state.known_entities.remove(entity.id);
            events.push(EntityChange::Destroy { id: entity.id });
        }
    }

    for id in update.loaded() {
        let cell = view.cell(id);

        for entity in cell.iter() {
            // Note that it is possible that an entity changed cells in
            // the exact same snapshot as the player. In this case the entity
            // must not be recreated. This check also handles the player entity
            // itself.
            if state.known_entities.contains(entity.id) {
                continue;
            }

            events.push(EntityChange::Create {
                entity: entity.clone(),
            });
        }
    }

    events
}

/// Update a player that hasn't moved cells.
fn update_player_cells<V>(view: V, state: &mut ConnectionState) -> Vec<EntityChange>
where
    V: AsView,
{
    let mut events = Vec::new();

    let mut stale_entities: HashSet<_> = state.known_entities.entities.keys().copied().collect();

    for id in state.cells.iter() {
        let cell = view.cell(id);

        for entity in cell.iter() {
            if !state.known_entities.contains(entity.id) {
                state.known_entities.insert(entity.clone());

                events.push(EntityChange::Create {
                    entity: entity.clone(),
                });

                continue;
            }

            stale_entities.remove(&entity.id);

            let known = state.known_entities.get_mut(entity.id).unwrap();

            if known.transform.translation != entity.transform.translation {
                known.transform.translation = entity.transform.translation;

                events.push(EntityChange::Translate {
                    id: entity.id,
                    translation: entity.transform.translation,
                    cell: None,
                });
            }

            if known.transform.rotation != entity.transform.rotation {
                known.transform.rotation = entity.transform.rotation;

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
        let events = update_player_cells(&view, &mut state);

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
        update_player_cells(&view, &mut state);

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
                cell: _
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
        update_player_cells(&view, &mut state);

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
        update_player_cells(&view, &mut state);

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
        update_player_cells(&view, &mut state);

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
                cell: _
            }
        ));
    }
}
