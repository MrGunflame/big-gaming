use std::collections::VecDeque;

use bevy_ecs::component::Component;
use glam::{Quat, Vec3};

/// A movement event, i.e. an actor is moving towards desination.
#[derive(Copy, Clone, Debug, Default, Component)]
pub struct Movement {
    /// The point the entity is moving to.
    pub direction: Quat,
}

#[derive(Clone, Debug, Default, Component)]
pub struct RotateQueue(pub VecDeque<Rotate>);

#[derive(Copy, Clone, Debug, Component)]
pub struct Rotate {
    pub destination: Quat,
}

impl From<Quat> for Rotate {
    #[inline]
    fn from(value: Quat) -> Self {
        Self { destination: value }
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct Teleport {
    pub destination: Vec3,
}

impl From<Vec3> for Teleport {
    #[inline]
    fn from(value: Vec3) -> Self {
        Self { destination: value }
    }
}

#[derive(Copy, Clone, Debug, Default, Component)]
pub struct Jump;
