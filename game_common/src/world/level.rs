use std::sync::Arc;

use ahash::HashMap;
use bevy_ecs::system::Resource;
use parking_lot::RwLock;

use super::gen::flat::FlatGenerator;
use super::gen::Generator;
use super::{Cell, CellId};

/// A game level/world.
#[derive(Resource)]
pub struct Level {
    cells: RwLock<HashMap<CellId, Arc<Cell>>>,
    generator: Generator,
}

impl Level {
    pub fn new() -> Self {
        Self {
            cells: RwLock::default(),
            generator: Generator::from(FlatGenerator),
        }
    }

    pub fn load(&self, id: CellId) -> Arc<Cell> {
        let cells = self.cells.read();
        match cells.get(&id) {
            Some(cell) => return cell.clone(),
            None => (),
        }

        drop(cells);
        let mut cells = self.cells.write();

        let mut cell = Cell::new(id);
        self.generator.generate(&mut cell);

        let cell = Arc::new(cell);
        cells.insert(id, cell.clone());
        cell.clone()
    }
}
