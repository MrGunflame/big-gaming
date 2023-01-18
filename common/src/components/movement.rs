use bevy_ecs::component::Component;
use glam::{Quat, Vec3};

/// A movement event, i.e. an actor is moving towards desination.
#[derive(Copy, Clone, Debug, Default, Component)]
pub struct Movement {
    /// The point the entity is moving to.
    pub direction: Quat,
}

#[derive(Copy, Clone, Debug, Component)]
pub struct Rotate {
    pub destination: Quat,
}

#[derive(Copy, Clone, Debug, Component)]
pub struct Teleport {
    pub destination: Vec3,
}

#[derive(Copy, Clone, Debug, Default, Component)]
pub struct Jump;
