use std::collections::VecDeque;

use ahash::HashMap;
use game_common::components::actions::ActionId;
use game_common::components::{PlayerId, Transform};
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event, EventQueue, PlayerConnect, PlayerDisconnect};
use game_common::world::control_frame::ControlFrame;
use game_common::world::hierarchy::update_global_transform;
use game_common::world::CellId;
use game_core::modules::Modules;
use game_net::message::{
    ControlMessage, DataMessage, DataMessageBody, Message, MessageId, SpawnHost,
};
use game_net::peer_error;
use game_script::effect::{Effect, Effects};
use game_script::{Context, WorldProvider};
use glam::Vec3;

use crate::conn::{Connection, Connections};
use crate::net::state::Cells;
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

    update_global_transform(&mut state.world.world);

    if cfg!(feature = "physics") {
        step_physics(state);
    }

    // Push snapshots last always
    let cf = state.state.control_frame.get();
    update_snapshots(&state.state.conns, &state.world, &state.level, cf);
}

fn apply_effects(effects: Effects, world: &mut WorldState, level: &mut Level) {
    // Since the script executing uses its own temporary ID namespace
    // for newly created IDs we must remap all IDs into "real" IDs.
    // A temporary ID must **never** overlap with an existing ID.
    // FIXME: We should use a linear IDs here so we can avoid
    // the need for hasing and just use array indexing.
    let mut entity_id_remap = HashMap::default();
    let mut resource_id_remap = HashMap::default();

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
            Effect::CreateResource(effect) => {
                debug_assert!(resource_id_remap.get(&effect.id).is_none());
                debug_assert!(world.world().get_resource(effect.id).is_none());

                let temp_id = effect.id;
                let real_id = world.world.insert_resource(effect.data);
                resource_id_remap.insert(temp_id, real_id);
            }
            Effect::DestroyResource(effect) => {
                let id = resource_id_remap
                    .get(&effect.id)
                    .copied()
                    .unwrap_or(effect.id);
                world.world.remove_resource(id);
            }
            Effect::UpdateResource(effect) => {
                let id = resource_id_remap
                    .get(&effect.id)
                    .copied()
                    .unwrap_or(effect.id);
                world.world.insert_resource_with_id(effect.data, id);
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
    let control_frame = state.state.control_frame.get();

    for conn in state.state.conns.iter() {
        let mut state = conn.state().write();

        // The const client interpolation delay.
        let client_cf = control_frame - state.peer_delay;

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
            Message::Control(ControlMessage::Ack(_)) => {}
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
                    DataMessageBody::ResourceCreate(_) => (),
                    DataMessageBody::ResourceDestroy(_) => (),
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

    queue.push(Event::Action(ActionEvent {
        entity,
        invoker: entity,
        action,
        data: data.clone(),
    }));
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

    let mut active_entity_changed = match state.host.entity {
        Some(entity) => entity != host_id,
        None => true,
    };

    let transform = world.get::<Transform>(host_id);
    let cell_id = CellId::from(transform.translation);

    let streamer = level.get_streamer(host_id).unwrap();

    // If the client requested a full update we must send him the entire
    // state in the current frame.
    if state.full_update {
        // We send a full update to the client by "forgetting" all of our
        // state of the client.
        // TODO: We can make this more efficient, because we only have to
        // spawn new entities and don't need to update/despawn any entities.
        state.entities.clear();
        state.known_entities.clear();
        active_entity_changed = true;

        state.full_update = false;
    }

    if state.cells.origin() != cell_id {
        tracing::info!("Moving host from {:?} to {:?}", state.cells, cell_id);

        state.cells.set(cell_id, streamer.distance);
    }

    let mut events = crate::net::sync_player(world, &mut state);

    if active_entity_changed {
        state.host.entity = Some(host_id);
        let id = state.entities.get(host_id).unwrap();
        events.push(DataMessageBody::SpawnHost(SpawnHost { entity: id }));
    }

    // ACKs need to be sent out before the actual data frames
    // in the control frame. If we were to sent the data before
    // a client with a low buffer might render the new state before
    // removing the predicted input for the frame.
    for id in conn.take_messages_in_frame() {
        conn.handle().acknowledge(id, cf);
    }

    for body in events {
        // FIXME: What to do if the send buffer is full?
        let _ = conn.handle().send(DataMessage {
            id: MessageId(0),
            control_frame: cf,
            body,
        });
    }

    conn.handle().set_cf(cf);
}
