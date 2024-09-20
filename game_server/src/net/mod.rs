use ahash::HashSet;
use game_common::components::components::RawComponent;
use game_common::components::{Global, GlobalTransform};
use game_common::entity::EntityId;
use game_common::net::ServerResource;
use game_common::world::{CellId, World};
use game_net::message::{
    DataMessageBody, EntityComponentAdd, EntityComponentRemove, EntityDestroy, ResourceCreate,
    ResourceDestroy,
};
use tracing::trace_span;

use crate::plugins::TickEvent;
use crate::world::state::WorldState;

use self::entities::Entities;
use self::state::ConnectionState;

pub mod entities;
pub mod state;

/// Synchronize a player to the current `world`.
pub(crate) fn sync_player(
    world: &WorldState,
    state: &mut ConnectionState,
    tick_events: &[TickEvent],
    new_cell: CellId,
    streamer_distance: u32,
) -> Vec<DataMessageBody> {
    let _span = trace_span!("sync_player").entered();

    let mut events = Vec::new();

    if state.cells.origin() != new_cell {
        tracing::info!("Moving host from {:?} to {:?}", state.cells, new_cell);

        let old_cells: HashSet<_> = state.cells.cells().into_iter().copied().collect();
        state.cells.set(new_cell, streamer_distance);
        let new_cells: HashSet<_> = state.cells.cells().into_iter().copied().collect();

        // Despawn all entities in cells that are streamed out.
        for cell in old_cells.difference(&new_cells) {
            for entity in world.cell(*cell).entities() {
                // `Global` entities are always replicated to the client.
                // We must retain them even if their cell is streamed out.
                if let Ok(Global) = world.world.get_typed(entity) {
                    continue;
                }

                let Some(server_entity) = state.entities.remove(entity) else {
                    continue;
                };

                events.push(DataMessageBody::EntityDestroy(EntityDestroy {
                    entity: server_entity,
                }));
            }
        }

        // Spawn in all entities in cells that are streamed in.
        for cell in new_cells.difference(&old_cells) {
            for entity in world.cell(*cell).entities() {
                // Entities referencing this entity may have already caused it
                // to be spawned.
                let server_entity = match state.entities.get(entity) {
                    Some(server_entity) => server_entity,
                    None => state.entities.insert(entity),
                };

                for (id, component) in world.world.components(entity).iter() {
                    let Some(component) = remap_component(&mut state.entities, component.clone())
                    else {
                        continue;
                    };

                    events.push(DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                        entity: server_entity,
                        component_id: id,
                        component,
                    }));
                }
            }
        }
    }

    for event in tick_events {
        match event {
            TickEvent::EntitySpawn(entity) => {
                let mut should_spawn = false;

                if let Ok(Global) = world.world.get_typed(*entity) {
                    should_spawn = true;
                }

                if let Ok(GlobalTransform(transform)) = world.world.get_typed(*entity) {
                    should_spawn |= state.cells.contains(CellId::from(transform.translation));
                }

                // If the client moved in this frame and the entity was spawned
                // into a cell that was just streamed in the entity was already
                // spawned. Don't spawn it twice.
                if should_spawn && !state.entities.contains(*entity) {
                    state.entities.insert(*entity);
                }
            }
            TickEvent::EntityDespawn(entity) => {
                let Some(server_entity) = state.entities.remove(*entity) else {
                    continue;
                };

                events.push(DataMessageBody::EntityDestroy(EntityDestroy {
                    entity: server_entity,
                }));
            }
            TickEvent::EntityComponentInsert(entity, id) => {
                let Some(server_entity) = state.entities.get(*entity) else {
                    continue;
                };

                let Some(component) = world.world.get(*entity, *id) else {
                    continue;
                };

                let Some(component) = remap_component(&mut state.entities, component.clone())
                else {
                    continue;
                };

                events.push(DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                    entity: server_entity,
                    component_id: *id,
                    component,
                }));
            }
            TickEvent::EntityComponentRemove(entity, id) => {
                let Some(server_entity) = state.entities.get(*entity) else {
                    continue;
                };

                events.push(DataMessageBody::EntityComponentRemove(
                    EntityComponentRemove {
                        entity: server_entity,
                        component: *id,
                    },
                ));
            }
            TickEvent::ResourceCreate(id) | TickEvent::ResourceUpdate(id) => {
                let Some(data) = world.world.get_resource(*id) else {
                    continue;
                };

                events.push(DataMessageBody::ResourceCreate(ResourceCreate {
                    id: ServerResource(id.to_bits()),
                    data: data.to_vec(),
                }));
            }
            TickEvent::ResourceDestroy(id) => {
                events.push(DataMessageBody::ResourceDestroy(ResourceDestroy {
                    id: ServerResource(id.to_bits()),
                }));
            }
        }
    }

    events
}

pub fn full_update(state: &mut ConnectionState, world: &World) -> Vec<DataMessageBody> {
    let _span = trace_span!("full_update").entered();

    state.entities.clear();
    // state.known_entities.clear();
    // state.known_resources.clear();

    let mut events = Vec::new();

    for (id, data) in world.iter_resources() {
        events.push(DataMessageBody::ResourceCreate(ResourceCreate {
            id: ServerResource(id.to_bits()),
            data: data.to_vec(),
        }));
    }

    for entity in world.entities() {
        let mut should_sync = false;

        if let Ok(Global) = world.get_typed(entity) {
            should_sync = true;
        }

        if let Ok(GlobalTransform(transform)) = world.get_typed(entity) {
            should_sync |= state.cells.contains(CellId::from(transform.translation));
        }

        if !should_sync {
            continue;
        }

        // Entities referencing this entity may have already caused it
        // to be spawned.
        let server_entity = match state.entities.get(entity) {
            Some(server_entity) => server_entity,
            None => state.entities.insert(entity),
        };

        for (id, component) in world.components(entity).iter() {
            let Some(component) = remap_component(&mut state.entities, component.clone()) else {
                continue;
            };

            events.push(DataMessageBody::EntityComponentAdd(EntityComponentAdd {
                entity: server_entity,
                component_id: id,
                component,
            }));
        }
    }

    events
}

fn remap_component(entities: &mut Entities, component: RawComponent) -> Option<RawComponent> {
    component
        .remap(|entity| {
            let server_entity = match entities.get(entity) {
                Some(server_entity) => server_entity,
                // FIXME: This will "spawn" the entity for the client, causing the
                // current component to stay valid.
                // However this does not fully synchronize the spawned entity if it
                // is outside the streaming area of the client.
                // This may cause surprising state desynchronizations with the client,
                // with "half synchronized" entities.
                None => entities.insert(entity),
            };

            Some(EntityId::from_raw(server_entity.0))
        })
        .ok()
}
