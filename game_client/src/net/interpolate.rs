use std::time::Instant;

use bevy::prelude::{Commands, Entity, Query, Transform};
use game_common::components::entity::InterpolateTranslation;

pub fn interpolate_translation(
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Transform, &InterpolateTranslation)>,
) {
    let now = Instant::now();

    for (entity, mut transform, interpolate) in &mut entities {
        transform.translation = interpolate.get(now);

        if now >= interpolate.end {
            commands.entity(entity).remove::<InterpolateTranslation>();
        }
    }
}
