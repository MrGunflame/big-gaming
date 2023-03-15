use bevy::prelude::{Commands, Entity, Query, Transform, With};

use crate::components::Player;

/// Despawn "lost" entities, i.e. entities that are above the maximum bounds
pub fn despawn_lost(mut commands: Commands, mut entities: Query<(Entity, &Transform)>) {
    for (entity, transform) in &mut entities {
        if transform.translation.y < -100.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn chunk_load(mut commands: Commands, mut entities: Query<(Entity, &Transform), With<Player>>) {
    for (entity, transform) in &entities {}
}
