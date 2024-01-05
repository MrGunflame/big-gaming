use std::collections::VecDeque;

use ahash::{HashMap, HashSet};
use game_common::components::actions::ActionId;
use game_common::components::components::{Components, RawComponent};
use game_common::components::inventory::Inventory;
use game_common::components::items::Item;
use game_common::components::Transform;
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event, EventQueue};
use game_common::net::ServerEntity;
use game_common::units::Mass;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::{CellId, World};
use game_core::modules::Modules;
use game_net::message::{
    ControlMessage, DataMessage, DataMessageBody, EntityComponentAdd, EntityComponentRemove,
    EntityComponentUpdate, EntityCreate, EntityDestroy, EntityRotate, EntityTranslate,
    InventoryItemAdd, InventoryItemRemove, InventoryItemUpdate, Message, MessageId, SpawnHost,
};
use game_net::peer_error;
use game_net::proto::components::ComponentRemove;
use game_script::effect::{Effect, Effects};
use game_script::{Context, WorldProvider};
use glam::Vec3;

use crate::conn::{Connection, Connections};
use crate::net::state::{Cells, ConnectionState, KnownEntities};
use crate::world::level::{Level, Streamer};
use crate::world::player::spawn_player;
use crate::world::state::WorldState;
use crate::ServerState;

// All systems need to run sequentially.
pub fn tick(state: &mut ServerState) {
    update_client_heads(state);
    flush_command_queue(state);

    crate::world::level::update_level_cells(state);

    // Send update event to every entity.
    for entity in state.world.world.iter() {
        state.event_queue.push(Event::Update(entity));
    }

    let effects = state.script_executor.update(Context {
        world: &state.world,
        physics: &state.pipeline,
        events: &mut state.event_queue,
        records: &state.modules,
    });
    apply_effects(effects, &mut state.world);

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

fn apply_effects(effects: Effects, world: &mut WorldState) {
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
            Effect::EntityComponentInsert(entity_id, component, data) => {
                let entity_id = entity_id_remap
                    .get(&entity_id)
                    .copied()
                    .unwrap_or(entity_id);

                world
                    .world
                    .insert(entity_id, component, RawComponent::new(data));
            }
            Effect::EntityComponentRemove(entity_id, component) => {
                world.world.remove(entity_id, component);
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
                let id = spawn_player(&srv_state.modules, world, &mut srv_state.scene).unwrap();

                // At the connection time the delay must be 0, meaning the player is spawned
                // without delay.
                debug_assert_eq!(state.peer_delay, ControlFrame(0));

                state.host.entity = Some(id);
                state.peer_delay = ControlFrame(0);
                state.cells = Cells::new(CellId::from(Vec3::ZERO));
                srv_state
                    .level
                    .create_streamer(id, Streamer { distance: 2 });
            }
            Message::Control(ControlMessage::Disconnected) => {}
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
) {
    tracing::info!("{:?} wants to run action {:?}", entity, action);

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

    let Some(host_id) = state.host.entity else {
        return;
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

    let events = update_player_cells(world, &mut state);

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

    for id in state.cells.cells() {
        let cell = world.cell(*id);

        for entity in cell.entities() {
            let entity_id = state.entities.insert(entity);
            state
                .known_entities
                .components
                .insert(entity, Components::new());

            // Sync all components.
            for (id, component) in world.world().components(entity).iter() {
                conn.handle().send(DataMessage {
                    id: MessageId(0),
                    control_frame: cf,
                    body: DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                        entity: entity_id,
                        component: id,
                        bytes: component.as_bytes().to_vec(),
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

/// Update a player that hasn't moved cells.
fn update_player_cells(world: &WorldState, state: &mut ConnectionState) -> Vec<DataMessageBody> {
    let mut events = Vec::new();

    let mut stale_entities: HashSet<_> = state.known_entities.components.keys().copied().collect();

    for id in state.cells.iter() {
        let cell = world.cell(id);

        for entity in cell.entities() {
            if !state.known_entities.contains(entity) {
                let entity_id = state.entities.insert(entity);
                state
                    .known_entities
                    .components
                    .insert(entity, Components::new());

                // Sync components.
                for (id, component) in world.world().components(entity).iter() {
                    events.push(DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                        entity: entity_id,
                        component: id,
                        bytes: component.as_bytes().to_vec(),
                    }));

                    state.known_entities.insert(entity, id, component.clone());
                }

                // Sync inventory.
                if let Some(inventory) = world.inventory(entity) {
                    for (id, stack) in inventory.iter() {
                        events.push(DataMessageBody::InventoryItemAdd(InventoryItemAdd {
                            entity: entity_id,
                            id,
                            item: stack.item.id,
                            quantity: stack.quantity,
                            components: stack.item.components.clone(),
                            equipped: stack.item.equipped,
                            hidden: stack.item.hidden,
                        }));

                        state
                            .known_entities
                            .inventories
                            .insert(entity, inventory.clone());
                    }
                }

                continue;
            }

            stale_entities.remove(&entity);

            let entity_id = state.entities.get(entity).unwrap();

            events.extend(update_components(
                entity_id,
                entity,
                world.world().components(entity),
                &state
                    .known_entities
                    .components
                    .get(&entity)
                    .unwrap()
                    .clone(),
                &mut state.known_entities,
            ));

            // Sync inventory
            match (
                world.inventory(entity),
                state.known_entities.inventories.get(&entity),
            ) {
                (Some(server_inv), Some(client_inv)) => {
                    events.extend(update_inventory(entity_id, server_inv, client_inv))
                }
                (Some(server_inv), None) => {
                    for (id, stack) in server_inv.iter() {
                        events.push(DataMessageBody::InventoryItemAdd(InventoryItemAdd {
                            entity: entity_id,
                            id,
                            item: stack.item.id,
                            quantity: stack.quantity,
                            components: stack.item.components.clone(),
                            equipped: stack.item.equipped,
                            hidden: stack.item.hidden,
                        }));
                    }
                }
                (None, Some(client_inv)) => {
                    for (id, _) in client_inv.iter() {
                        events.push(DataMessageBody::InventoryItemRemove(InventoryItemRemove {
                            entity: entity_id,
                            slot: id,
                        }));
                    }
                }
                (None, None) => (),
            }

            if let Some(inventory) = world.inventory(entity) {
                state
                    .known_entities
                    .inventories
                    .insert(entity, inventory.clone());
            } else {
                state.known_entities.inventories.remove(&entity);
            }
        }
    }

    // Despawn all entities that were not existent in any of the player's cells.
    for id in stale_entities {
        state.known_entities.despawn(id);
        let entity_id = state.entities.remove(id).unwrap();
        events.push(DataMessageBody::EntityDestroy(EntityDestroy {
            entity: entity_id,
        }));
    }

    events
}

fn update_components(
    entity: ServerEntity,
    entity_id: EntityId,
    server_state: &Components,
    client_state: &Components,
    known_state: &mut KnownEntities,
) -> Vec<DataMessageBody> {
    let mut events = Vec::new();

    for (id, component) in server_state.iter() {
        if client_state.get(id).is_none() {
            events.push(DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                entity,
                component: id,
                bytes: component.as_bytes().to_vec(),
            }));

            known_state
                .components
                .entry(entity_id)
                .or_default()
                .insert(id, component.clone());

            continue;
        }

        let server_component = component;
        let client_component = client_state.get(id).unwrap();

        if server_component != client_component {
            events.push(DataMessageBody::EntityComponentUpdate(
                EntityComponentUpdate {
                    entity,
                    component: id,
                    bytes: server_component.as_bytes().to_vec(),
                },
            ));

            known_state
                .components
                .get_mut(&entity_id)
                .unwrap()
                .insert(id, component.clone());
        }
    }

    for (id, _) in client_state
        .iter()
        .filter(|(id, _)| server_state.get(*id).is_none())
    {
        events.push(DataMessageBody::EntityComponentRemove(
            EntityComponentRemove {
                entity,
                component: id,
            },
        ));

        known_state
            .components
            .get_mut(&entity_id)
            .unwrap()
            .remove(id);
    }

    events
}

fn update_inventory(
    entity_id: ServerEntity,
    server_state: &Inventory,
    client_state: &Inventory,
) -> Vec<DataMessageBody> {
    let mut events = Vec::new();

    for (id, server_stack) in server_state.iter() {
        let Some(client_stack) = client_state.get(id) else {
            events.push(DataMessageBody::InventoryItemAdd(InventoryItemAdd {
                entity: entity_id,
                id,
                item: server_stack.item.id,
                quantity: server_stack.quantity,
                components: server_stack.item.components.clone(),
                equipped: server_stack.item.equipped,
                hidden: server_stack.item.hidden,
            }));

            continue;
        };

        // This should never actually happen since we don't allow modification
        // of the item id once inserted. This is only available via removal and
        // re-insertion.
        if server_stack.item.id != client_stack.item.id {
            panic!("Server-side state inventory state missmatch");
        }

        // We need to send an update if the equipped/hidden state or the stack
        // quantity changed.
        let mut needs_update = false;

        if server_stack.item.equipped != client_stack.item.equipped
            || server_stack.item.hidden != client_stack.item.hidden
        {
            needs_update = true;
        }

        let mut quantity = None;
        if server_stack.quantity != client_stack.quantity {
            needs_update = true;
            quantity = Some(server_stack.quantity);
        }

        let mut components = None;
        if server_stack.item.components != client_stack.item.components {
            needs_update = true;
            components = Some(server_stack.item.components.clone());
        }

        if needs_update {
            events.push(DataMessageBody::InventoryItemUpdate(InventoryItemUpdate {
                entity: entity_id,
                slot: id,
                equipped: server_stack.item.equipped,
                hidden: server_stack.item.hidden,
                quantity,
                components,
            }));
        }
    }

    for (id, _) in client_state
        .iter()
        .filter(|(id, _)| server_state.get(*id).is_none())
    {
        events.push(DataMessageBody::InventoryItemRemove(InventoryItemRemove {
            entity: entity_id,
            slot: id,
        }))
    }

    events
}

#[cfg(test)]
mod tests {
    use game_common::components::object::ObjectId;
    use game_common::components::Transform;
    use game_common::entity::EntityId;
    use game_common::record::RecordReference;
    use game_common::world::entity::{Entity, EntityBody, Object};
    use game_common::world::snapshot::EntityChange;
    use game_common::world::CellId;
    use glam::{IVec3, Vec3};

    use crate::net::state::ConnectionState;
    use crate::world::state::WorldState;

    use super::update_player_cells;

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
