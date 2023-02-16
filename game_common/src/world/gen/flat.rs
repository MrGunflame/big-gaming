use bevy_transform::prelude::Transform;

use crate::components::object::ObjectId;
use crate::world::entity::Object;
use crate::world::gen::Generate;
use crate::world::Cell;

pub struct FlatGenerator;

impl Generate for FlatGenerator {
    fn generate(&self, cell: &mut Cell) {
        for _ in 0..10 {}

        cell.spawn(Object::builder().id(ObjectId(0.into())).build());
    }
}
