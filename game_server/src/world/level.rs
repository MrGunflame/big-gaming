use ahash::{HashMap, HashSet};
use game_common::components::{Decode, Transform};
use game_common::entity::EntityId;
use game_common::events::{CellLoad, Event};
use game_common::world::cell::square;
use game_common::world::CellId;
use game_prefab::Prefab;
use game_wasm::components::Component;
use game_worldgen::WorldgenState;

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
            tracing::info!("generating cell {:?}", cell);

            for entity in state.level.generator.load(*cell) {
                let Some(module) = state.modules.get(entity.prefab.module) else {
                    continue;
                };

                let Some(record) = module.records.get(entity.prefab.record) else {
                    continue;
                };

                let prefab = match Prefab::from_bytes(&record.data) {
                    Ok(prefab) => prefab,
                    Err(err) => {
                        tracing::error!("failed to decode prefab record: {}", err);
                        continue;
                    }
                };

                prefab.instantiate(&mut state.world.world);
            }

            state.level.loaded.insert(*cell);
            state
                .event_queue
                .push(Event::CellLoad(CellLoad { id: *cell }));
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
        let transform = Transform::decode(transform.reader()).unwrap();

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
    generator: WorldgenState,
}

impl Level {
    pub fn new(generator: WorldgenState) -> Self {
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
