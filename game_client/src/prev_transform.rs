use bevy::prelude::{Component, Query, Transform};

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct PreviousTransform(pub Transform);

pub fn update_previous_transform(mut entities: Query<(&Transform, &mut PreviousTransform)>) {
    for (transform, mut previous_transform) in &mut entities {
        previous_transform.0 = *transform;
    }
}
