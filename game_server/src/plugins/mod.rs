mod inventory;

use std::collections::VecDeque;

use ahash::HashSet;
use game_common::events::{ActionEvent, Event};
use game_common::world::control_frame::ControlFrame;
use game_common::world::snapshot::EntityChange;
use game_common::world::source::StreamingSource;
use game_common::world::world::{AsView, WorldState, WorldViewRef};
use game_common::world::CellId;
use game_net::message::{
    ControlMessage, DataMessage, DataMessageBody, EntityCreate, EntityDestroy, EntityRotate,
    EntityTranslate, Message, SpawnHost,
};
use game_script::Context;

use crate::conn::{Connection, Connections};
use crate::net::state::{Cells, ConnectionState};
use crate::world::player::spawn_player;
use crate::ServerState;

// All systems need to run sequentially.
pub fn tick(state: &mut ServerState) {
    update_client_heads(state);
    flush_command_queue(state);

    crate::world::level::update_level_cells(state);

    state.script_executor.run(Context {
        view: &mut state.world.back_mut().unwrap(),
        physics_pipeline: &state.pipeline,
        events: &mut state.event_queue,
    });

    if cfg!(feature = "physics") {
        //step_physics(state);
    }

    // Push snapshots last always
    update_snapshots(&state.state.conns, &state.world);
}

fn step_physics(state: &mut ServerState) {
    let start = state.world.front().unwrap().control_frame();
    let end = state.world.back().unwrap().control_frame();

    state
        .pipeline
        .step(&mut state.world, start, end, &mut state.event_queue);
}

fn update_client_heads(state: &mut ServerState) {
    let control_frame = state.state.control_frame.lock();

    state.world.insert(*control_frame);

    for conn in state.state.conns.iter() {
        let mut state = conn.state().write();

        // The const client interpolation delay.
        let client_cf = *control_frame - state.peer_delay;

        state.client_cf = client_cf;
    }

    if state.world.len() > 120 {
        state.world.pop();
    }
}

fn flush_command_queue(srv_state: &mut ServerState) {
    let mut queue = VecDeque::new();
    for conn in srv_state.state.conns.iter() {
        while let Some(msg) = conn.handle().recv() {
            queue.push_back((conn.id(), msg));
        }
    }

    while let Some((id, msg)) = queue.pop_front() {
        let conn = srv_state.state.conns.get(id).unwrap();
        let client_cf = conn.state().read().client_cf;

        // Fetch the world state at the client's computed render time.
        // Note that the client may be too far in the past to go back.
        // In that case we must chose the oldest snapshot.
        let mut view;
        {
            let opt = srv_state.world.get_mut(client_cf);
            if let Some(v) = opt {
                view = v;
            } else {
                // Note that this `drop` is necessary as `Option<WorldViewMut>` has a `Drop`
                // impl, even thought at this point it never actually needs to drop anything
                // because it is `None`.
                drop(opt);
                match srv_state.world.front_mut() {
                    Some(v) => view = v,
                    None => {
                        tracing::warn!("no snapshots");
                        return;
                    }
                }
            }
        }

        let mut state = conn.state().write();

        match msg {
            Message::Control(ControlMessage::Connected()) => {
                let res = spawn_player(&mut view);

                state.entities.insert(res.id);

                view.insert_streaming_source(
                    res.id,
                    StreamingSource {
                        distance: srv_state.state.config.player_streaming_source_distance,
                    },
                );

                // At the connection time the delay must be 0, meaning the player is spawned
                // without delay.
                debug_assert_eq!(state.peer_delay, ControlFrame(0));

                state.host.entity = Some(res.id);
                state.peer_delay = ControlFrame(0);
                state.cells = Cells::new(CellId::from(res.transform.translation));
            }
            Message::Control(ControlMessage::Disconnected) => {}
            Message::Data(msg) => match msg.body {
                DataMessageBody::EntityCreate(msg) => {}
                DataMessageBody::EntityDestroy(msg) => {
                    if let Some(id) = state.host.entity {
                        if view.despawn(id).is_none() {
                            tracing::warn!("attempted to destroy an unknown entity {:?}", id);
                        }
                    }

                    // Remove the player from the connections ref.
                    srv_state.state.conns.remove(id);
                }
                DataMessageBody::EntityTranslate(msg) => {
                    let Some(id) = state.entities.get(msg.entity) else {
                        continue;
                    };

                    let Some(mut entity) = view.get_mut(id) else {
                        continue;
                    };

                    entity.set_translation(msg.translation);
                }
                DataMessageBody::EntityRotate(msg) => {
                    let Some(id) = state.entities.get(msg.entity) else {
                        continue;
                    };

                    let Some(mut entity) = view.get_mut(id) else {
                        continue;
                    };

                    entity.set_rotation(msg.rotation);
                }
                DataMessageBody::EntityAction(msg) => {
                    let Some(entity) = state.entities.get(msg.entity) else {
                        continue;
                    };

                    // TODO: Validate that the peer has the acton.
                    srv_state.event_queue.push(Event::Action(ActionEvent {
                        entity,
                        invoker: entity,
                        action: msg.action,
                    }));
                }
                DataMessageBody::SpawnHost(_) => (),
            },
        }

        drop(view);
    }
}

fn update_snapshots(
    connections: &Connections,
    // FIXME: Make dedicated type for all shared entities.
    // mut entities: Query<(&mut Entity, &Transform)>,
    world: &WorldState,
) {
    let Some(view) = world.back() else {
        return;
    };

    // tracing::info!("Sending snapshots for {:?}", view.control_frame());

    for conn in connections.iter() {
        update_client(&conn, view);
    }
}

fn update_client(conn: &Connection, view: WorldViewRef<'_>) {
    let state = &mut *conn.state().write();

    let Some(host_id) = state.host.entity else {
        return;
    };

    let host = view.get(host_id).unwrap();
    let cell_id = CellId::from(host.transform.translation);

    let streaming_source = view.streaming_sources().get(host_id).unwrap();

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
            let cell = view.cell(*id);

            for entity in cell.iter() {
                state.known_entities.insert(entity.clone());

                let entity_id = state.entities.insert(entity.id);

                conn.handle().send_cmd(DataMessage {
                    control_frame: view.control_frame(),
                    body: DataMessageBody::EntityCreate(EntityCreate {
                        entity: entity_id,
                        translation: entity.transform.translation,
                        rotation: entity.transform.rotation,
                        data: entity.body.clone(),
                    }),
                });

                // Sync the entity inventory, if it has one.
                if let Some(inventory) = view.inventories().get(entity.id) {
                    for item in inventory.iter() {
                        todo!()
                    }
                }
            }
        }

        // Also sent the host.
        let id = state.entities.get(host_id).unwrap();
        conn.handle().send_cmd(DataMessage {
            control_frame: view.control_frame(),
            body: DataMessageBody::SpawnHost(SpawnHost { entity: id }),
        });

        return;
    }

    // `Cells::set` may allocate so avoid calling it unless
    // necessary.
    if state.cells.origin() != cell_id {
        tracing::info!("Moving host from {:?} to {:?}", state.cells, cell_id);

        state.cells.set(cell_id, streaming_source.distance);
    }

    let events = update_player_cells(view, state);

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

    let control_frame = view.control_frame();
    for body in update_client_entities(state, events) {
        let msg = DataMessage {
            control_frame,
            body,
        };
        conn.handle().send_cmd(msg);
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
                dbg!(entity.transform.translation);
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

fn update_client_entities(
    state: &mut ConnectionState,
    events: Vec<EntityChange>,
) -> Vec<DataMessageBody> {
    let mut cmds = Vec::new();

    for event in events {
        let cmd = match event {
            EntityChange::Create { entity } => {
                let entity_id = state.entities.insert(entity.id);
                state.known_entities.insert(entity.clone());

                DataMessageBody::EntityCreate(EntityCreate {
                    entity: entity_id,
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    data: entity.body,
                })
            }
            EntityChange::Destroy { id } => {
                let entity_id = state.entities.remove(id).unwrap();
                state.known_entities.remove(id);

                DataMessageBody::EntityDestroy(EntityDestroy { entity: entity_id })
            }
            EntityChange::Translate { id, translation } => {
                let entity_id = state.entities.get(id).unwrap();
                let entity = state.known_entities.get_mut(id).unwrap();

                entity.transform.translation = translation;

                DataMessageBody::EntityTranslate(EntityTranslate {
                    entity: entity_id,
                    translation,
                })
            }
            EntityChange::Rotate { id, rotation } => {
                let entity_id = state.entities.get(id).unwrap();
                let entity = state.known_entities.get_mut(id).unwrap();

                entity.transform.rotation = rotation;

                DataMessageBody::EntityRotate(EntityRotate {
                    entity: entity_id,
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
            is_host: false,
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
        entity.set_translation(Vec3::splat(1.0));
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
        entity.set_translation(Vec3::splat(1024.0));
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
        entity.set_translation(new_cell.min());
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
