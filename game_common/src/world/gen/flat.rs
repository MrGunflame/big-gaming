use bevy_transform::prelude::Transform;

use crate::components::object::{Object, ObjectId};
use crate::world::gen::Generate;
use crate::world::Cell;

pub struct FlatGenerator;

impl Generate for FlatGenerator {
    fn generate(&self, cell: &mut Cell) {
        for _ in 0..10 {}

        // cell.spawn()
        //     .insert(Transform::from_translation(Vec3))
        //     .insert(Object {
        //         id: ObjectId(0.into()),
        //     });

        dbg!(cell.id);
    }
}
