use ahash::{HashMap, HashSet, HashSetExt};
use game_common::components::{Global, Transform};
use game_common::entity::EntityId;
use game_common::events::{CellLoad, CellUnload, Event};
use game_common::world::cell::square;
use game_common::world::{CellId, World};
use game_core::modules::Modules;
use game_prefab::Prefab;
use game_worldgen::WorldgenState;
use tracing::trace_span;

#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub fn update_level_cells(level: &mut Level, world: &mut World, modules: &Modules) -> Vec<Event> {
    let mut cells = HashSet::default();
    let mut events = Vec::new();

    for (entity, streamer) in &level.streamers {
        let Ok(transform) = world.get_typed::<Transform>(*entity) else {
            continue;
        };

        let cell = CellId::from(transform.translation);

        let area = square(cell, streamer.distance);
        cells.extend(area);
    }

    for cell in &cells {
        // If the cell already is in the set of currently loaded cells
        // we don't have to do anything.
        if level.loaded.contains(cell) {
            continue;
        }

        // Otherwise we must create the cell and instantiate all prefab
        // entities as defined by the cell generator.
        tracing::info!("generating cell {:?}", cell);

        for entity in level.generator.load(*cell) {
            let Some(module) = modules.get(entity.prefab.module) else {
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

            prefab.instantiate(&mut *world);
        }

        level.loaded.insert(*cell);
        events.push(Event::CellLoad(CellLoad { id: *cell }));

        tracing::info!("loading cell {:?}", cell);
    }

    // Unload cells before scheduling new cells to be unloaded.
    level.unload_cell(world);

    // Cells that are no longer in the active area of a streamer must be unloaded.
    // Note that we don't actually do any unloading here since we want scripts to
    // be able to handle the `CellUnload` event that happens just before the cell
    // is actually unloaded.
    // This means unloading is always delayed by one tick.
    for cell in level.loaded.difference(&cells) {
        events.push(Event::CellUnload(CellUnload { id: *cell }));
        level.unload_in_next_tick.insert(*cell);
    }

    level.loaded = cells;

    events
}

pub struct Level {
    loaded: HashSet<CellId>,
    streamers: HashMap<EntityId, Streamer>,
    generator: WorldgenState,
    unload_in_next_tick: HashSet<CellId>,
}

impl Level {
    pub fn new(generator: WorldgenState) -> Self {
        Self {
            loaded: HashSet::default(),
            streamers: HashMap::default(),
            generator,
            unload_in_next_tick: HashSet::new(),
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

    /// Unloads all entities from cells that are scheduled for unloading.
    fn unload_cell(&mut self, world: &mut World) {
        let _span = trace_span!("Level::unload_cell").entered();

        // For most frames no cells will be unloaded. If there
        // are no cells scheduled to be unloaded we don't need
        // to query the world for entities within those cells.
        if self.unload_in_next_tick.is_empty() {
            return;
        }

        if cfg!(debug_assertions) {
            for cell in &self.unload_in_next_tick {
                assert!(!self.loaded.contains(cell));
            }
        }

        let mut despawn_queue = Vec::new();
        for entity in world.entities() {
            // Entities with a `Global` component are always loaded.
            if let Ok(Global) = world.get_typed::<Global>(entity) {
                continue;
            }

            let Ok(transform) = world.get_typed::<Transform>(entity) else {
                continue;
            };

            let cell = CellId::from(transform.translation);
            if self.unload_in_next_tick.contains(&cell) {
                despawn_queue.push(entity);
            }
        }

        tracing::debug!("unloading {} entities", despawn_queue.len());
        for entity in despawn_queue {
            world.despawn(entity);
        }

        self.unload_in_next_tick.clear();
    }
}

#[cfg(test)]
mod tests {
    use ahash::{HashSet, HashSetExt};
    use game_common::components::Transform;
    use game_common::events::Event;
    use game_common::world::cell::square;
    use game_common::world::{CellId, World};
    use game_core::modules::Modules;
    use game_worldgen::WorldgenState;
    use glam::Vec3;

    use super::{update_level_cells, Level, Streamer};

    #[test]
    fn test_update_level_cells() {
        let mut level = Level::new(WorldgenState::new());
        let mut world = World::new();
        let modules = Modules::new();

        let events = update_level_cells(&mut level, &mut world, &modules);
        assert!(events.is_empty());
        assert!(level.loaded.is_empty());

        let player = world.spawn();
        let mut transform = Transform::default();
        world.insert_typed(player, transform);
        level.create_streamer(player, Streamer { distance: 1 });

        let loaded_cells = square(CellId::from(transform.translation), 1)
            .into_iter()
            .collect();
        let events = update_level_cells(&mut level, &mut world, &modules);
        assert_cell_load_events(&events, &loaded_cells);
        assert_cell_unload_events(&events, &HashSet::new());
        assert_eq!(level.loaded, loaded_cells);

        // Move to neighboring cell.
        transform.translation -= Vec3::new(1.0, 0.0, 0.0);
        world.insert_typed(player, transform);

        let loaded_cells_2: HashSet<_> = square(CellId::from(transform.translation), 1)
            .into_iter()
            .collect();
        let new_loaded_cells: HashSet<_> =
            loaded_cells_2.difference(&loaded_cells).copied().collect();
        let new_unloaded_cells: HashSet<_> =
            loaded_cells.difference(&loaded_cells_2).copied().collect();
        let events = update_level_cells(&mut level, &mut world, &modules);
        assert_cell_load_events(&events, &new_loaded_cells);
        assert_cell_unload_events(&events, &new_unloaded_cells);
        assert_eq!(level.loaded, loaded_cells_2);

        // Destroy the player, unloading all cells.
        world.despawn(player);
        level.destroy_streamer(player);
        let events = update_level_cells(&mut level, &mut world, &modules);
        assert_cell_load_events(&events, &HashSet::new());
        assert_cell_unload_events(&events, &loaded_cells_2);
        assert!(level.loaded.is_empty());
    }

    #[track_caller]
    fn assert_cell_load_events(actual: &[Event], expected: &HashSet<CellId>) {
        // Order does not matter so we collect both
        // slices into HashSets.

        let actual: HashSet<_> = actual
            .into_iter()
            .filter_map(|e| match e {
                Event::CellLoad(e) => Some(e.id),
                _ => None,
            })
            .collect();

        assert_eq!(actual, *expected);
    }

    #[track_caller]
    fn assert_cell_unload_events(actual: &[Event], expected: &HashSet<CellId>) {
        let actual: HashSet<_> = actual
            .into_iter()
            .filter_map(|e| match e {
                Event::CellUnload(e) => Some(e.id),
                _ => None,
            })
            .collect();

        assert_eq!(actual, *expected);
    }
}
