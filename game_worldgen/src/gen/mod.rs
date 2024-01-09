use game_common::world::entity::Terrain;
use game_common::world::gen::{CellBuilder, EntityBuilder, Generate};

use crate::data::Cells;

pub struct StaticGenerator {
    pub data: Cells,
}

impl Generate for StaticGenerator {
    fn generate(&self, cell: &mut CellBuilder) {
        let Some(entities) = self.data.cells.get(&cell.id()) else {
            return;
        };

        for entity in entities {
            // if let Some(terrain) = &entity.terrain {
            //     let builder = EntityBuilder::default().terrain(Terrain {
            //         mesh: terrain.clone(),
            //     });

            //     cell.spawn(builder);
            //     continue;
            // }

            let mut builder = EntityBuilder::new().transform(entity.transform);
            for (id, component) in entity.components.iter() {
                builder = builder.component(id, component.clone());
            }

            cell.spawn(builder);
        }
    }
}
