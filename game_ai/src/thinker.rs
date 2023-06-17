use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, Query};
use game_common::components::transform::Transform;
use glam::Vec3;

use crate::actions::Rotate;
use crate::sense::Vision;

pub(super) fn thinkers(app: &mut App) {
    app.add_system(think);
}

fn think(mut commands: Commands, hosts: Query<(Entity, &Transform, &Vision)>) {
    for (entity, transform, vision) in &hosts {
        let Some(target) = vision.entities.first() else {
            continue;
        };

        // If both source and target rotations are equal, only bad things will happen (NaNs).
        // This is essentially unreachable for normal gameplay purposes (as actors can
        // cannot stack).
        if transform.rotation == target.rotation {
            continue;
        }

        let new = transform.looking_at(target.translation, Vec3::Y);

        commands.entity(entity).insert(Rotate {
            target: new.rotation,
        });
    }
}
