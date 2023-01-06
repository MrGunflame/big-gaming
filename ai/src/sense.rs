use bevy::prelude::{App, Query, Transform, With};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use common::components::actor::Actor;

pub(super) fn senses(app: &mut App) {
    app.add_system(vision_sense);
}

#[derive(Clone, Debug, Default, Component)]
pub struct Vision {
    pub entities: Vec<Transform>,
}

impl Vision {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

fn vision_sense(
    mut hosts: Query<(Entity, &Transform, &mut Vision)>,
    mut actors: Query<(Entity, &Transform), With<Actor>>,
) {
    for (entity, host, mut vision) in &mut hosts {
        for (actor, actor_transform) in &mut actors {
            // Skip the host itself.
            if entity == actor {
                continue;
            }

            vision.entities.clear();

            let distance = actor_transform.translation - host.translation;

            if distance.length() < 10.0 {
                vision.entities.push(*actor_transform);
            }
        }
    }
}
