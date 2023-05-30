use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Commands, Query};
use game_common::components::object::Lifetime;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(despawn_expired_objects);
    }
}

fn despawn_expired_objects(mut commands: Commands, mut objects: Query<(Entity, &Lifetime)>) {
    for (entity, lifetime) in &mut objects {
        if !lifetime.is_expired() {
            continue;
        }

        commands.entity(entity).despawn();
    }
}
