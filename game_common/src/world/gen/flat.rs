use bevy_transform::prelude::Transform;
use glam::Vec3;

use crate::components::object::ObjectId;
use crate::world::entity::Object;
use crate::world::gen::Generate;
use crate::world::Cell;

pub struct FlatGenerator;

impl Generate for FlatGenerator {
    fn generate(&self, cell: &mut Cell) {
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
