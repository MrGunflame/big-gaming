use ahash::{HashMap, HashSet};
use bevy_ecs::system::Resource;
use game_common::world::cell::{square, Cell};
use game_common::world::gen::flat::FlatGenerator;
use game_common::world::gen::Generator;
use game_common::world::world::WorldState;
use game_common::world::CellId;

pub fn update_level_cells(world: &mut WorldState, level: &mut Level) {
    let Some(mut view) = world.back_mut() else {
        return;
    };

    let mut cells = HashSet::default();

    for (id, source) in view.streaming_sources().iter() {
        let entity = view.get(id).unwrap();
        let cell = CellId::from(entity.transform.translation);

        let area = square(cell, source.distance);
        cells.extend(area);
    }

    for cell in &cells {
        // If the cell is already loaded, don't update
        // anything.
        if level.loaded.contains(cell) {
            level.loaded.remove(cell);
            continue;
        }

        if !level.cells.contains_key(cell) {
            let mut cell = Cell::new(*cell);
            level.generator.generate(&mut cell);
            level.cells.insert(cell.id(), cell);
        }

        tracing::info!("loading cell {:?}", cell);

        let cell = level.cells.get_mut(cell).unwrap();
        cell.load(&mut view);
    }

    for cell in &level.loaded {
        tracing::info!("unloading cell {:?}", cell);

        let cell = level.cells.get_mut(cell).unwrap();
        cell.unload(&mut view);
    }

    level.loaded = cells;
}

#[derive(Resource)]
pub struct Level {
    loaded: HashSet<CellId>,
    cells: HashMap<CellId, Cell>,
    generator: Generator,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            loaded: HashSet::default(),
            cells: HashMap::default(),
            generator: Generator::from(FlatGenerator),
        }
    }
}
