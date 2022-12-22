//! Actor components

use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};

use bevy_ecs::component::Component;

/// An entity that may act on its own within the world, i.e. players and NPCs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Actor;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct ActorState(NonZeroU32);

impl ActorState {
    pub const DEFAULT: Self = Self(NonZeroU32::new(1).unwrap());
    pub const DEAD: Self = Self(NonZeroU32::new(2).unwrap());

    pub fn is_default(self) -> bool {
        self == Self::DEFAULT
    }
}

impl Default for ActorState {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The movement speed of an actor, in meter/second.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Component)]
#[repr(transparent)]
pub struct MovementSpeed(pub f32);

impl Deref for MovementSpeed {
    type Target = f32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MovementSpeed {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
