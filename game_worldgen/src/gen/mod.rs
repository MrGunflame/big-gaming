use game_common::world::gen::{CellBuilder, EntityBuilder, Generate};

use crate::data::Cells;

pub struct StaticGenerator {
    data: Cells,
}

impl Generate for StaticGenerator {
    fn generate(&self, cell: &mut CellBuilder) {
        let Some(entities) = self.data.cells.get(&cell.id()) else {
            return;
        };

        for entity in entities {
            let mut builder = EntityBuilder::new(entity.id);
            for (id, component) in entity.components.iter() {
                builder = builder.component(id, component.clone());
            }

            cell.spawn(builder);
        }
    }
}
