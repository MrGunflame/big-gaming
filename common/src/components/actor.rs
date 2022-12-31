//! Actor components

use std::num::{NonZeroU32, NonZeroU8};
use std::ops::{Deref, DerefMut};

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

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

/// A [`Limb`] of an [`Actor`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct ActorLimb {
    /// The actor who owns the limb.
    pub actor: Entity,
    pub limb: Limb,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Limb(pub NonZeroU8);

impl Limb {
    #[inline]
    pub const fn new(id: u8) -> Self {
        Self(NonZeroU8::new(id).unwrap())
    }

    #[inline]
    pub const unsafe fn new_unchecked(id: u8) -> Self {
        unsafe { Self(NonZeroU8::new_unchecked(id)) }
    }
}
