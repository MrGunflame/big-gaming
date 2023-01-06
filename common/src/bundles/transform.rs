use bevy_ecs::bundle::Bundle;
use bevy_transform::components::{GlobalTransform, Transform};

use crate::components::transform::PreviousTransform;

#[derive(Copy, Clone, Debug, Default, Bundle)]
pub struct TransformBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub previous_transform: PreviousTransform,
}

impl TransformBundle {
    pub const fn new(transform: Transform) -> Self {
        Self {
            transform,
            global_transform: GlobalTransform::IDENTITY,
            previous_transform: PreviousTransform(transform),
        }
    }
}
