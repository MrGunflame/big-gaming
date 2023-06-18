use bevy_ecs::prelude::Component;
use game_common::components::combat::Health;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;

#[derive(Clone, Debug, Component)]
pub struct LoadActor {
    pub transform: Transform,
    pub race: RaceId,
    pub health: Health,
}
