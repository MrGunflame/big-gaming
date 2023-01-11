use bevy_ecs::component::Component;
use glam::Vec3;

/// A movement event, i.e. an actor is moving towards desination.
#[derive(Copy, Clone, Debug, Default, Component)]
pub struct Movement {
    /// The point the entity is moving to.
    pub desination: Vec3,
}
