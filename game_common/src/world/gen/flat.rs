use bevy_transform::prelude::Transform;
use glam::Vec3;
use noise::{NoiseFn, Simplex};

use crate::components::object::ObjectId;
use crate::world::entity::Object;
use crate::world::gen::Generate;
use crate::world::terrain::{Heightmap, TerrainMesh};
use crate::world::{Cell, CELL_SIZE, CELL_SIZE_UINT};

pub struct FlatGenerator;

impl Generate for FlatGenerator {
    fn generate(&self, cell: &mut Cell) {
        let noise = Simplex::default();

        let mut map = Heightmap::default();

        for index in 0..(CELL_SIZE_UINT.x + 1) * (CELL_SIZE_UINT.z + 1) {
            let x = cell.id.min_x() as u32 + (index % (CELL_SIZE_UINT.x + 1));
            let z = cell.id.min_z() as u32 + (index / (CELL_SIZE_UINT.z + 1));

            dbg!(x);

            let y = noise.get([x as f64 / 20.0, z as f64 / 20.0]);
            map.nodes.push(y as f32 * 2.0 as f32);
        }

        cell.spawn(TerrainMesh::new(cell.id, map));

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
