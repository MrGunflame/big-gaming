use bevy::prelude::{Query, Transform};
use game_common::components::transform::PreviousTransform;

pub fn update_previous_transform(mut entities: Query<(&Transform, &mut PreviousTransform)>) {
    for (transform, mut previous_transform) in &mut entities {
        previous_transform.0 = *transform;
    }
}
