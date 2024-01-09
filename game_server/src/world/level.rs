use ahash::{HashMap, HashSet};
use game_common::components::{Component, Decode, Transform};
use game_common::entity::EntityId;
use game_common::world::cell::square;
use game_common::world::gen::{CellBuilder, Generator};
use game_common::world::CellId;

use crate::ServerState;

#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub fn update_level_cells(state: &mut ServerState) {
    let mut cells = HashSet::default();

    for (entity, streamer) in &state.level.streamers {
        let transform = state.world.get::<Transform>(*entity);
        let cell = CellId::from(transform.translation);

        let area = square(cell, streamer.distance);
        cells.extend(area);
    }

    for cell in &cells {
        // If the cell is already loaded, don't update
        // anything.
        if state.level.loaded.contains(cell) {
            state.level.loaded.remove(cell);
            continue;
        }

        if !state.level.loaded.contains(cell) {
            let mut builder = CellBuilder::new(*cell);
            state.level.generator.generate(&mut builder);

            for builder in builder.into_entities() {
                let id = state.world.spawn();
                for (component_id, component) in builder.components.iter() {
                    state
                        .world
                        .world
                        .insert(id, component_id, component.clone());
                }
            }

            state.level.loaded.insert(*cell);
        }

        tracing::info!("loading cell {:?}", cell);
    }

    for cell in &state.level.loaded {
        // TODO: Unload cell
    }

    state.level.loaded = cells;

    // TODO: We would like to delay entity despawning for a frame and signal
    // the entity that it is being unloaded in the next frame to do any
    // required cleanup.
    let mut despawn_queue = Vec::new();
    for id in state.world.keys() {
        let Some(transform) = state.world.world.get(id, Transform::ID) else {
            continue;
        };
        let transform = Transform::decode(transform.as_bytes()).unwrap();

        let cell = CellId::from(transform.translation);
        // Despawn all entities that have moved outside of any loaded cells.
        if !state.level.loaded.contains(&cell) {
            tracing::debug!("unloading entity {:?}", id);
            despawn_queue.push(id);
        }
    }

    for id in despawn_queue {
        state.world.world.despawn(id);
    }
}

pub struct Level {
    loaded: HashSet<CellId>,
    streamers: HashMap<EntityId, Streamer>,
    generator: Generator,
}

impl Level {
    pub fn new(generator: Generator) -> Self {
        Self {
            loaded: HashSet::default(),
            streamers: HashMap::default(),
            generator,
        }
    }

    pub fn create_streamer(&mut self, id: EntityId, streamer: Streamer) {
        self.streamers.insert(id, streamer);
    }

    pub fn destroy_streamer(&mut self, id: EntityId) {
        self.streamers.remove(&id);
    }

    pub fn get_streamer(&self, id: EntityId) -> Option<&Streamer> {
        self.streamers.get(&id)
    }
}
