use bevy::prelude::{App, Commands, Quat, Query, Transform, Vec3};
use bevy_ecs::entity::Entity;

use crate::actions::Rotate;
use crate::sense::Vision;

pub(super) fn thinkers(app: &mut App) {
    app.add_system(think);
}

fn think(mut commands: Commands, hosts: Query<(Entity, &Vision)>) {
    for (entity, vision) in &hosts {
        let Some(target) = vision.entities.first() else {
            continue;
        };

        commands.entity(entity).insert(Rotate {
            rotation: target.rotation,
        });
    }
}
