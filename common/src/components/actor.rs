//! Actor components

use std::num::{NonZeroU32, NonZeroU8};
use std::ops::{Deref, DerefMut, Mul};
use std::time::Duration;

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use glam::Vec3;

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

impl Mul<Duration> for MovementSpeed {
    type Output = f32;

    fn mul(self, rhs: Duration) -> Self::Output {
        self.0 * rhs.as_secs_f32()
    }
}

/// A model of an [`Actor`], including multi-entity meshes and animations.
///
/// **Note that not every [`Actor`] has this component.**
#[derive(Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct ActorModel {
    /// The entities that make up the actor model.
    pub entities: Box<[Entity]>,
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

#[derive(Copy, Clone, Debug, Component)]
pub struct ActorFigure {
    /// The offset to the eyes.
    ///
    /// This is where the first-person camera should be placed.
    pub eyes: Vec3,
}

/// A spawning point for an actor.
#[derive(Copy, Clone, Debug)]
pub struct SpawnPoint {
    /// The point that this `SpawnPoint` refers to.
    pub location: Vec3,
    /// The weight that this `SpawnPoint` has. An actor usually spawns at the point with the
    /// heighest weight.
    pub weight: u32,
}

/// A list of [`SpawnPoint`]s.
#[derive(Clone, Debug, Component)]
pub struct SpawnPoints {
    // FIXME: This might better be a BTree.
    points: Vec<SpawnPoint>,
}

impl SpawnPoints {
    #[inline]
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Pushes a new [`SpawnPoint`] into the collection.
    pub fn push(&mut self, point: SpawnPoint) {
        self.points.push(point);

        // The point with the highest weight at the front.
        self.points
            .sort_by(|a, b| a.weight.cmp(&b.weight).reverse());
    }

    /// Returns the heighest rated [`SpawnPoint`].
    #[inline]
    pub fn highest(&self) -> Option<SpawnPoint> {
        self.points.first().copied()
    }
}
