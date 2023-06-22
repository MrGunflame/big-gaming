use bevy_ecs::bundle::Bundle;

use crate::components::transform::{GlobalTransform, PreviousTransform, Transform};

#[derive(Copy, Clone, Debug, Default, Bundle)]
pub struct TransformBundle {
    pub transform: Transform,
    pub previous_transform: PreviousTransform,
    pub global_transform: GlobalTransform,
}

impl TransformBundle {
    pub const fn new(transform: Transform) -> Self {
        Self {
            transform,
            previous_transform: PreviousTransform(transform),
            global_transform: GlobalTransform(transform),
        }
    }
}
