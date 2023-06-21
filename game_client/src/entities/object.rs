use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query};
use game_common::components::object::ObjectId;
use game_common::components::transform::Transform;

#[derive(Clone, Debug, Component)]
pub struct LoadObject {
    pub id: ObjectId,
    pub transform: Transform,
}

pub fn load_object(mut commands: Commands, entities: Query<(Entity, &LoadObject)>) {
    for (entity, object) in &entities {
        tracing::trace!("spawning object at {:?}", object.transform.translation);
    }
}
