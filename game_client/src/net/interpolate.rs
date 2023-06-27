use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Commands, Query, Res};
use game_common::components::entity::InterpolateTranslation;
use game_common::components::transform::Transform;
use game_core::time::Time;

pub fn interpolate_translation(
    time: Res<Time>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Transform, &InterpolateTranslation)>,
) {
    let now = time.last_update();

    for (entity, mut transform, interpolate) in &mut entities {
        let now = now - (interpolate.end - interpolate.start);

        transform.translation = interpolate.get(now);

        if now >= interpolate.end {
            commands.entity(entity).remove::<InterpolateTranslation>();
        }
    }
}
