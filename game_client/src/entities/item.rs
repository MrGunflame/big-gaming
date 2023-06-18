use bevy_ecs::prelude::Component;
use game_common::components::items::ItemId;
use game_common::components::transform::Transform;

#[derive(Clone, Debug, Component)]
pub struct LoadItem {
    pub id: ItemId,
    pub transform: Transform,
}
