use ahash::HashSet;
use game_common::components::components::Components;
use game_common::entity::EntityId;
use game_common::net::ServerEntity;
use game_net::message::{
    DataMessageBody, EntityComponentAdd, EntityComponentRemove, EntityComponentUpdate,
    EntityDestroy,
};
use tracing::trace_span;

use crate::world::state::WorldState;

use self::entities::Entities;
use self::state::ConnectionState;

pub mod entities;
pub mod state;

/// Synchronize a player to the current `world`.
pub fn sync_player(world: &WorldState, state: &mut ConnectionState) -> Vec<DataMessageBody> {
    let _span = trace_span!("sync_player").entered();

    let mut events = Vec::new();

    // Entities that were known to the client in the previous tick.
    let mut prev_entities: HashSet<_> = state.known_entities.components.keys().copied().collect();

    for cell_id in state.cells.iter() {
        let cell = world.cell(cell_id);

        for entity in cell.entities() {
            prev_entities.remove(&entity);

            if state.entities.get(entity).is_none() {
                state.entities.insert(entity);
                state.known_entities.spawn(entity);
            }
        }
    }

    for cell_id in state.cells.iter() {
        let cell = world.cell(cell_id);

        for entity in cell.entities() {
            let entity_id = state.entities.get(entity).unwrap();

            let server_state = world.world.components(entity);
            let client_state = state.known_entities.components.get_mut(&entity).unwrap();
            events.extend(sync_components(
                entity_id,
                client_state,
                server_state,
                &state.entities,
            ));
        }
    }

    for entity in prev_entities {
        state.known_entities.despawn(entity);
        let server_entity = state.entities.remove(entity).unwrap();

        events.push(DataMessageBody::EntityDestroy(EntityDestroy {
            entity: server_entity,
        }));
    }

    events
}

/// Synchronize the current server components into the client components for the given entity.
fn sync_components(
    entity: ServerEntity,
    client_state: &mut Components,
    server_state: &Components,
    entities: &Entities,
) -> Vec<DataMessageBody> {
    let mut events = Vec::new();

    for (id, component) in server_state.iter() {
        // FIXME: It is possible for a component to refer to an entity that is
        // loaded but outside the loaded area around the client, i.e. it is not
        // actually synchronized to the client. We just ignore the entity for
        // now.
        let Ok(component) = component
            .clone()
            .remap(|id| entities.get(id).map(|id| EntityId::from_raw(id.0)))
        else {
            continue;
        };

        // Component does not exist on client.
        if client_state.get(id).is_none() {
            client_state.insert(id, component.clone());

            events.push(DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                entity,
                component_id: id,
                component,
            }));

            continue;
        }

        // Component exists on server and client.
        let server_component = component;
        let client_component = client_state.get(id).unwrap();

        if &server_component != client_component {
            client_state.insert(id, server_component.clone());

            events.push(DataMessageBody::EntityComponentUpdate(
                EntityComponentUpdate {
                    entity,
                    component_id: id,
                    component: server_component,
                },
            ));
        }
    }

    for (id, _) in client_state.clone().iter() {
        if server_state.get(id).is_none() {
            client_state.remove(id);
        }
    }

    // Component exists on client but not on server.
    client_state.retain(|id, _| {
        if server_state.get(id).is_none() {
            events.push(DataMessageBody::EntityComponentRemove(
                EntityComponentRemove {
                    entity,
                    component: id,
                },
            ));

            false
        } else {
            true
        }
    });

    events
}
