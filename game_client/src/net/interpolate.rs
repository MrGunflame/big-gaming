use std::time::Instant;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Commands, Query};
use game_common::components::entity::InterpolateTranslation;
use game_common::components::transform::Transform;

pub fn interpolate_translation(
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Transform, &InterpolateTranslation)>,
) {
    let now = Instant::now();

    for (entity, mut transform, interpolate) in &mut entities {
        let now = now - (interpolate.end - interpolate.start);

        transform.translation = interpolate.get(now);

        if now >= interpolate.end {
            commands.entity(entity).remove::<InterpolateTranslation>();
        }
    }
}
