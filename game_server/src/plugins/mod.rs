use std::collections::VecDeque;

use ahash::{HashMap, HashSet};
use game_common::components::actions::ActionId;
use game_common::components::components::Components;
use game_common::components::inventory::Inventory;
use game_common::components::{PlayerId, Transform};
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event, EventQueue, PlayerConnect, PlayerDisconnect};
use game_common::net::ServerEntity;
use game_common::world::control_frame::ControlFrame;
use game_common::world::CellId;
use game_core::modules::Modules;
use game_net::message::{
    ControlMessage, DataMessage, DataMessageBody, EntityComponentAdd, EntityComponentRemove,
    EntityComponentUpdate, EntityDestroy, InventoryItemAdd, InventoryItemRemove,
    InventoryItemUpdate, Message, MessageId, SpawnHost,
};
use game_net::peer_error;
use game_script::effect::{Effect, Effects};
use game_script::{Context, WorldProvider};
use glam::Vec3;

use crate::conn::{Connection, Connections};
use crate::net::entities::Entities;
use crate::net::state::{Cells, ConnectionState, KnownEntities};
use crate::world::level::{Level, Streamer};
use crate::world::state::WorldState;
use crate::ServerState;

// All systems need to run sequentially.
pub fn tick(state: &mut ServerState) {
    update_client_heads(state);
    flush_command_queue(state);

    crate::world::level::update_level_cells(state);

    let effects = state.script_executor.update(Context {
        world: &state.world,
        physics: &state.pipeline,
        events: &mut state.event_queue,
        records: &state.modules,
    });
    apply_effects(effects, &mut state.world, &mut state.level);

    if cfg!(feature = "physics") {
        step_physics(state);
    }

    // Push snapshots last always
    let cf = *state.state.control_frame.lock();
    update_snapshots(&state.state.conns, &state.world, &state.level, cf);

    // state
    //     .scene
    //     .spawner
    //     .update(&mut state.scene.graph, &state.pool, None);
    // state.scene.graph.compute_transform();
    // state.scene.graph.clear_trackers();
}

fn apply_effects(effects: Effects, world: &mut WorldState, level: &mut Level) {
    // Since the script executing uses its own temporary ID namespace
    // for newly created IDs we must remap all IDs into "real" IDs.
    // A temporary ID must **never** overlap with an existing ID.
    // FIXME: We should use a linear IDs here so we can avoid
    // the need for hasing and just use array indexing.
    let mut entity_id_remap = HashMap::default();
    let mut inventory_slot_id_remap = HashMap::default();

    for effect in effects.into_iter() {
        match effect {
            Effect::EntitySpawn(id) => {
                debug_assert!(entity_id_remap.get(&id).is_none());
                debug_assert!(!world.world().contains(id));

                let temp_id = id;
                let real_id = world.spawn();
                entity_id_remap.insert(temp_id, real_id);
            }
            Effect::EntityDespawn(id) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let entity = world.remove(id);
            }
            Effect::InventoryInsert(id, temp_slot_id, stack) => {
                let entity_id = entity_id_remap.get(&id).copied().unwrap_or(id);

                let real_id = world.inventory_mut(entity_id).insert(stack);
                inventory_slot_id_remap.insert(temp_slot_id, real_id);
            }
            Effect::InventoryRemove(id, slot_id, quantity) => {
                let entity_id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                world.inventory_mut(entity_id).remove(slot_id);
            }
            Effect::InventoryItemUpdateEquip(id, slot_id, equipped) => {
                let entity_id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                world
                    .inventory_mut(entity_id)
                    .get_mut(slot_id)
                    .set_equipped(equipped);
            }
            Effect::InventoryComponentInsert(id, slot_id, component, data) => {
                let entity_id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                world
                    .inventory_mut(entity_id)
                    .get_mut(slot_id)
                    .component_insert(component, data);
            }
            Effect::InventoryComponentRemove(id, slot_id, component) => {
                let entity_id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                world
                    .inventory_mut(entity_id)
                    .get_mut(slot_id)
                    .component_remove(component);
            }
            Effect::InventoryClear(entity_id) => {
                let entity_id = entity_id_remap
                    .get(&entity_id)
                    .copied()
                    .unwrap_or(entity_id);

                world.inventory_mut(entity_id).clear();
            }
            Effect::EntityComponentInsert(effect) => {
                let entity = entity_id_remap
                    .get(&effect.entity)
                    .copied()
                    .unwrap_or(effect.entity);

                let component = match effect.component.remap(|entity| {
                    match entity_id_remap.get(&entity).copied() {
                        Some(entity) => Some(entity),
                        None => {
                            if world.world.contains(entity) {
                                Some(entity)
                            } else {
                                None
                            }
                        }
                    }
                }) {
                    Ok(component) => component,
                    Err(err) => {
                        tracing::warn!("discarding invalid component: {}", err);
                        continue;
                    }
                };

                world.world.insert(entity, effect.component_id, component);
            }
            Effect::EntityComponentRemove(effect) => {
                let entity = entity_id_remap
                    .get(&effect.entity)
                    .copied()
                    .unwrap_or(effect.entity);

                world.world.remove(entity, effect.component_id);
            }
            Effect::PlayerSetActive(effect) => {
                let entity = entity_id_remap
                    .get(&effect.entity)
                    .copied()
                    .unwrap_or(effect.entity);

                if let Some(old_entity) = world.players.insert(effect.player, entity) {
                    level.destroy_streamer(old_entity);
                }

                level.create_streamer(entity, Streamer { distance: 2 });
            }
        }
    }
}

fn step_physics(state: &mut ServerState) {
    state
        .pipeline
        .step(&mut state.world.world, &mut state.event_queue);
}

fn update_client_heads(state: &mut ServerState) {
    let control_frame = state.state.control_frame.lock();

    for conn in state.state.conns.iter() {
        let mut state = conn.state().write();

        // The const client interpolation delay.
        let client_cf = *control_frame - state.peer_delay;

        state.client_cf = client_cf;
    }
}

fn flush_command_queue(srv_state: &mut ServerState) {
    let mut queue = VecDeque::new();
    for conn in srv_state.state.conns.iter() {
        while let Some(msg) = conn.handle().recv() {
            queue.push_back((conn.key(), msg));
        }
    }

    while let Some((id, msg)) = queue.pop_front() {
        let conn = srv_state.state.conns.get(id).unwrap();
        let client_cf = conn.state().read().client_cf;

        let mut state = conn.state().write();

        let world = &mut srv_state.world;

        match msg {
            Message::Control(ControlMessage::Connected()) => {
                let player = PlayerId::from_raw(srv_state.next_player);
                srv_state.next_player += 1;

                // At the connection time the delay must be 0, meaning the player is spawned
                // without delay.
                debug_assert_eq!(state.peer_delay, ControlFrame(0));

                state.host.player = Some(player);
                state.peer_delay = ControlFrame(0);
                state.cells = Cells::new(CellId::from(Vec3::ZERO));

                srv_state
                    .event_queue
                    .push(Event::PlayerConnect(PlayerConnect { player }));
            }
            Message::Control(ControlMessage::Disconnected) => {
                let player = state.host.player.unwrap();
                srv_state
                    .event_queue
                    .push(Event::PlayerDisconnect(PlayerDisconnect { player }));
            }
            Message::Control(ControlMessage::Acknowledge(_, _)) => {}
            Message::Data(msg) => {
                conn.push_message_in_frame(msg.id);

                match msg.body {
                    DataMessageBody::EntityDestroy(msg) => {
                        peer_error!("received server-only frame `EntityDestroy` from peer")
                    }
                    DataMessageBody::EntityTranslate(msg) => {
                        peer_error!("received server-only frame `EntityTranslate` from peer")
                    }
                    DataMessageBody::EntityRotate(msg) => {
                        let Some(id) = state.entities.get(msg.entity) else {
                            continue;
                        };

                        let mut transform = world.get::<Transform>(id);
                        transform.rotation = msg.rotation;
                        world.insert(id, transform);
                    }
                    DataMessageBody::EntityAction(msg) => {
                        let Some(entity) = state.entities.get(msg.entity) else {
                            continue;
                        };

                        if state.host.entity != Some(entity) {
                            peer_error!("peer tried to control entity it does not own");
                            continue;
                        }

                        queue_action(
                            &world,
                            entity,
                            &srv_state.modules,
                            msg.action,
                            &mut srv_state.event_queue,
                            msg.bytes,
                        );
                    }
                    DataMessageBody::EntityComponentAdd(_) => (),
                    DataMessageBody::EntityComponentRemove(_) => (),
                    DataMessageBody::EntityComponentUpdate(_) => (),
                    DataMessageBody::SpawnHost(_) => (),
                    DataMessageBody::InventoryItemAdd(_) => (),
                    DataMessageBody::InventoryItemRemove(_) => (),
                    DataMessageBody::InventoryItemUpdate(_) => (),
                }
            }
        }
    }
}

fn queue_action(
    world: &WorldState,
    entity: EntityId,
    modules: &Modules,
    action: ActionId,
    queue: &mut EventQueue,
    data: Vec<u8>,
) {
    tracing::info!(
        "{:?} wants to run action {:?} with params ({:?})",
        entity,
        action,
        data,
    );

    let components = world.world.components(entity);

    for (id, _) in components.iter() {
        let Some(component) = modules
            .get(id.module)
            .map(|module| module.records.get(id.record))
            .flatten()
            .map(|record| record.body.as_component())
            .flatten()
        else {
            continue;
        };

        if component.actions.contains(&action.0) {
            tracing::trace!("found action {:?} on component", action);

            queue.push(Event::Action(ActionEvent {
                entity,
                invoker: entity,
                action,
                data: data.clone(),
            }));
        }
    }

    let Some(inventory) = world.inventory(entity) else {
        return;
    };

    for (_, stack) in inventory.iter().filter(|(_, stack)| stack.item.equipped) {
        let item_id = stack.item.id;

        let Some(item) = modules
            .get(item_id.0.module)
            .map(|module| module.records.get(item_id.0.record))
            .flatten()
            .map(|record| record.body.as_item())
            .flatten()
        else {
            return;
        };

        if item.actions.contains(&action.0) {
            tracing::trace!("found action {:?} on item", action);

            queue.push(Event::Action(ActionEvent {
                entity: entity,
                invoker: entity,
                action,
                data,
            }));
            return;
        }

        for (id, _) in stack.item.components.iter() {
            let Some(component) = modules
                .get(id.module)
                .map(|module| module.records.get(id.record))
                .flatten()
                .map(|record| record.body.as_component())
                .flatten()
            else {
                return;
            };

            if component.actions.contains(&action.0) {
                tracing::trace!("found action {:?} on item component", action);

                queue.push(Event::Action(ActionEvent {
                    entity: entity,
                    invoker: entity,
                    action,
                    data,
                }));
                return;
            }
        }
    }

    tracing::trace!("action {:?} unavailable for entity {:?}", action, entity);
}

fn update_snapshots(
    connections: &Connections,
    world: &WorldState,
    level: &Level,
    cf: ControlFrame,
) {
    for conn in connections.iter() {
        update_client(&conn, world, level, cf);
    }
}

fn update_client(conn: &Connection, world: &WorldState, level: &Level, cf: ControlFrame) {
    let mut state = conn.state().write();

    let Some(player_id) = state.host.player else {
        return;
    };

    let Some(host_id) = world.players.get(&player_id).copied() else {
        return;
    };

    let active_entity_changed = match state.host.entity {
        Some(entity) => entity != host_id,
        None => true,
    };

    let transform = world.get::<Transform>(host_id);
    let cell_id = CellId::from(transform.translation);

    let streamer = level.get_streamer(host_id).unwrap();

    // Send full state
    // The delta from the current frame is "included" in the full update.
    if state.full_update {
        state.full_update = false;
        drop(state);
        send_full_update(conn, world, host_id, cf);
        return;
    }

    // `Cells::set` may allocate so avoid calling it unless
    // necessary.
    if state.cells.origin() != cell_id {
        tracing::info!("Moving host from {:?} to {:?}", state.cells, cell_id);

        state.cells.set(cell_id, streamer.distance);
    }

    let events = crate::net::sync_player(world, &mut state);

    // ACKs need to be sent out before the actual data frames
    // in the control frame. If we were to sent the data before
    // a client with a low buffer might render the new state before
    // removing the predicted input for the frame.
    for id in conn.take_messages_in_frame() {
        conn.handle().acknowledge(id, cf);
    }

    for body in events {
        let msg = DataMessage {
            id: MessageId(0),
            control_frame: cf,
            body,
        };
        conn.handle().send(msg);
    }

    if active_entity_changed {
        state.host.entity = Some(host_id);
        let id = state.entities.get(host_id).unwrap();
        conn.handle().send(DataMessage {
            id: MessageId(0),
            control_frame: cf,
            body: DataMessageBody::SpawnHost(SpawnHost { entity: id }),
        });
    }
}

fn send_full_update(conn: &Connection, world: &WorldState, host: EntityId, cf: ControlFrame) {
    let state = &mut *conn.state().write();

    let transform = world.get::<Transform>(host);
    let cell_id = CellId::from(transform.translation);

    state.entities.clear();
    state.known_entities.clear();

    tracing::info!(
        "sending full update to host in cell {:?} for cells: {:?}",
        cell_id,
        state.cells.cells(),
    );

    // Initialize entities before syncing components. Components
    // may refer to entities that are not yet loaded.
    for id in state.cells.cells() {
        let cell = world.cell(*id);

        for entity in cell.entities() {
            state.entities.insert(entity);
            state
                .known_entities
                .components
                .insert(entity, Components::new());
        }
    }

    for id in state.cells.cells() {
        let cell = world.cell(*id);

        for entity in cell.entities() {
            let entity_id = state.entities.get(entity).unwrap();

            // Sync all components.
            for (id, component) in world.world().components(entity).iter() {
                let component = component
                    .clone()
                    .remap(|entity| {
                        state
                            .entities
                            .get(entity)
                            .map(|id| EntityId::from_raw(id.0))
                    })
                    .unwrap();

                conn.handle().send(DataMessage {
                    id: MessageId(0),
                    control_frame: cf,
                    body: DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                        entity: entity_id,
                        component_id: id,
                        component: component.clone(),
                    }),
                });

                state.known_entities.insert(entity, id, component.clone());
            }

            // Sync the entity inventory, if it has one.
            if let Some(inventory) = world.inventory(entity) {
                for (id, stack) in inventory.iter() {
                    conn.handle().send(DataMessage {
                        id: MessageId(0),
                        control_frame: cf,
                        body: DataMessageBody::InventoryItemAdd(InventoryItemAdd {
                            entity: entity_id,
                            id,
                            quantity: stack.quantity,
                            item: stack.item.id,
                            components: stack.item.components.clone(),
                            equipped: stack.item.equipped,
                            hidden: stack.item.hidden,
                        }),
                    });
                }

                state
                    .known_entities
                    .inventories
                    .insert(entity, inventory.clone());
            }
        }
    }

    // Also sent the host.
    let id = state.entities.get(host).unwrap();
    conn.handle().send(DataMessage {
        id: MessageId(0),
        control_frame: cf,
        body: DataMessageBody::SpawnHost(SpawnHost { entity: id }),
    });
}

#[cfg(test)]
mod tests {
    use game_common::components::object::ObjectId;
    use game_common::components::Transform;
    use game_common::entity::EntityId;
    use game_common::record::RecordReference;
    use game_common::world::entity::{Entity, EntityBody, Object};
    use glam::Vec3;

    fn create_test_entity() -> Entity {
        Entity {
            id: EntityId::dangling(),
            transform: Transform::default(),
            body: EntityBody::Object(Object {
                id: ObjectId(RecordReference::STUB),
            }),
            components: Default::default(),
            is_host: false,
            linvel: Vec3::ZERO,
            angvel: Vec3::ZERO,
        }
    }

    // #[test]
    // fn player_update_cells_spawn_entity() {
    //     let mut world = WorldState::new();
    //     world.insert(create_test_entity());

    //     let mut state = ConnectionState::new();
    //     let events = update_player_cells(&world, &state);
    //     update_client_entities(&mut state, events.clone());

    //     assert_eq!(events.len(), 1);
    //     assert!(matches!(events[0], EntityChange::Create { entity: _ }));
    // }

    // #[test]
    // fn player_update_cells_translate_entity() {
    //     let mut world = WorldState::new();
    //     let entity_id = world.insert(create_test_entity());

    //     let mut state = ConnectionState::new();
    //     let events = update_player_cells(&world, &state);
    //     update_client_entities(&mut state, events);

    //     let entity = world.get_mut(entity_id).unwrap();
    //     entity.transform.translation = Vec3::splat(1.0);

    //     let events = update_player_cells(&world, &mut state);

    //     assert_eq!(events.len(), 1);
    //     assert!(matches!(
    //         events[0],
    //         EntityChange::Translate {
    //             id: _,
    //             translation: _,
    //         }
    //     ));
    // }

    // #[test]
    // fn player_upate_cells_despawn_entity() {
    //     let mut world = WorldState::new();
    //     let entity_id = world.insert(create_test_entity());

    //     let mut state = ConnectionState::new();
    //     let events = update_player_cells(&world, &state);
    //     update_client_entities(&mut state, events);

    //     world.remove(entity_id);

    //     let events = update_player_cells(&world, &mut state);

    //     assert_eq!(events.len(), 1);
    //     assert!(matches!(events[0], EntityChange::Destroy { id: _ }));
    // }

    // #[test]
    // fn player_update_cells_entity_leave_cells() {
    //     let mut world = WorldState::new();
    //     let entity_id = world.insert(create_test_entity());

    //     let mut state = ConnectionState::new();
    //     let events = update_player_cells(&world, &state);
    //     update_client_entities(&mut state, events);

    //     let entity = world.get_mut(entity_id).unwrap();
    //     entity.transform.translation = Vec3::splat(1024.0);

    //     let events = update_player_cells(&world, &mut state);

    //     assert_eq!(events.len(), 1);
    //     assert!(matches!(events[0], EntityChange::Destroy { id: _ }));
    // }

    // #[test]
    // fn player_update_cells_entity_translate_parallel() {
    //     let distance = 0;

    //     let mut world = WorldState::new();
    //     let entity_id = world.insert(create_test_entity());

    //     let mut state = ConnectionState::new();
    //     state.cells.set(CellId::from_i32(IVec3::new(0, 0, 0)), 0);
    //     let events = update_player_cells(&world, &mut state);
    //     update_client_entities(&mut state, events);

    //     let new_cell = CellId::from_i32(IVec3::splat(1));
    //     state.cells.set(new_cell, distance);

    //     let entity = world.get_mut(entity_id).unwrap();
    //     entity.transform.translation = new_cell.min();

    //     let events = update_player_cells(&world, &mut state);

    //     assert_eq!(events.len(), 1);
    //     assert!(matches!(
    //         events[0],
    //         EntityChange::Translate {
    //             id: _,
    //             translation: _,
    //         }
    //     ));
    // }
}
