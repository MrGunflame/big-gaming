use bevy_ecs::prelude::Bundle;
use bevy_transform::prelude::Transform;
use glam::{Quat, Vec3};

use crate::components::object::{LoadObject, ObjectId};

use super::TransformBundle;

#[derive(Bundle)]
pub struct ObjectBundle {
    #[bundle]
    pub transform: TransformBundle,
    pub object: LoadObject,
}

impl ObjectBundle {
    pub const fn new(id: ObjectId) -> Self {
        Self {
            transform: TransformBundle::new(Transform::IDENTITY),
            object: LoadObject { id },
        }
    }

    pub const fn translation(mut self, translation: Vec3) -> Self {
        self.transform.transform.translation = translation;
        self
    }

    pub const fn rotation(mut self, rotation: Quat) -> Self {
        self.transform.transform.rotation = rotation;
        self
    }
}
