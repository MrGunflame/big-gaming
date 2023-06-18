use bevy_ecs::prelude::Component;
use game_common::components::object::ObjectId;
use game_common::components::transform::Transform;

#[derive(Clone, Debug, Component)]
pub struct LoadObject {
    pub id: ObjectId,
    pub transform: Transform,
}
