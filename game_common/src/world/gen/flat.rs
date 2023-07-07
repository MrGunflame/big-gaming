use glam::UVec2;
use noise::{NoiseFn, Simplex};

use crate::record::RecordReference;
use crate::world::gen::Generate;
use crate::world::terrain::Heightmap;
use crate::world::CELL_SIZE_UINT;

use super::{CellBuilder, EntityBuilder};

pub struct FlatGenerator;

impl Generate for FlatGenerator {
    fn generate(&self, cell: &mut CellBuilder) {
        let noise = Simplex::default();

        let mut map = Vec::default();

        for index in 0..(CELL_SIZE_UINT.x + 1) * (CELL_SIZE_UINT.z + 1) {
            let x = (cell.id().min_x() as i32 + ((index % (CELL_SIZE_UINT.x + 1)) as i32)) as u32;
            let z = (cell.id().min_z() as i32 + (index / (CELL_SIZE_UINT.z + 1)) as i32) as u32;

            let y = noise.get([x as f64 / 20.0, z as f64 / 20.0]);
            map.push(y as f32 * 2.0 as f32);
        }

        // cell.spawn(TerrainMesh::new(
        //     cell.id,
        //     Heightmap::from_vec(UVec2::new(CELL_SIZE_UINT.x + 1, CELL_SIZE_UINT.z + 1), map),
        // ));

        let mesh = Heightmap::from_vec(UVec2::new(2, 2), vec![0.0, 0.0, 0.0, 0.0]);
        cell.spawn(EntityBuilder::new(RecordReference::STUB).terrain(mesh));

        // cell.spawn(Item {
        //     id: ItemId(0.into()),
        //     transform: Transform::from_translation(Vec3::splat(25.0)),
        //     components: Default::default(),
        // });

        // for _ in 0..10 {}

        // let mut x = cell.id.min_x();
        // while x < cell.id.max_x() {
        //     cell.spawn(
        //         Object::builder()
        //             .id(ObjectId(0.into()))
        //             .translation(Vec3::new(x, cell.id.min_y(), cell.id.min_z()))
        //             .build(),
        //     );

        //     x += 1.0;
        // }
    }
}
