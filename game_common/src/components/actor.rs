//! Actor components

use std::collections::HashSet;
use std::num::{NonZeroU32, NonZeroU8};
use std::ops::{Deref, DerefMut, Mul};
use std::time::Duration;

use glam::{Quat, Vec3};

/// An entity that may act on its own within the world, i.e. players and NPCs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Actor;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ActorFlag(NonZeroU32);

impl ActorFlag {
    /// Is the [`Actor`] dead?
    pub const DEAD: Self = Self(unsafe { NonZeroU32::new_unchecked(1) });

    /// Can the [`Actor`] move?
    pub const CAN_MOVE: Self = Self(unsafe { NonZeroU32::new_unchecked(16) });

    /// Can the [`Actor`] rotate?
    pub const CAN_ROTATE: Self = Self(unsafe { NonZeroU32::new_unchecked(17) });

    /// Can the [`Actor`] attack?
    pub const CAN_ATTACK: Self = Self(unsafe { NonZeroU32::new_unchecked(18) });
}

#[derive(Clone, Debug)]
pub struct ActorFlags {
    flags: HashSet<ActorFlag>,
}

impl ActorFlags {
    #[inline]
    pub fn new() -> Self {
        Self {
            flags: HashSet::new(),
        }
    }

    pub fn contains(&self, flag: ActorFlag) -> bool {
        self.flags.contains(&flag)
    }

    pub fn insert(&mut self, flag: ActorFlag) {
        self.flags.insert(flag);
    }

    pub fn remove(&mut self, flag: ActorFlag) {
        self.flags.remove(&flag);
    }
}

impl Default for ActorFlags {
    fn default() -> Self {
        // FIXME: These "default" flags should probably come from somewhere different.
        let mut flags = Self::new();
        flags.insert(ActorFlag::CAN_MOVE);
        flags.insert(ActorFlag::CAN_ROTATE);
        flags.insert(ActorFlag::CAN_ATTACK);
        flags
    }
}

/// The movement speed of an actor, in meter/second.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
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

impl Default for MovementSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Limb(pub NonZeroU8);

impl Limb {
    #[inline]
    pub fn new(id: u8) -> Self {
        Self(NonZeroU8::new(id).unwrap())
    }

    #[inline]
    pub const unsafe fn new_unchecked(id: u8) -> Self {
        unsafe { Self(NonZeroU8::new_unchecked(id)) }
    }
}

#[derive(Copy, Clone, Debug, Default)]
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
    pub translation: Vec3,
    /// The weight that this `SpawnPoint` has. An actor usually spawns at the point with the
    /// heighest weight.
    pub weight: u32,
}

/// A list of [`SpawnPoint`]s.
#[derive(Clone, Debug)]
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
    pub fn best(&self) -> Option<SpawnPoint> {
        self.points.first().copied()
    }
}

impl From<SpawnPoint> for SpawnPoints {
    #[inline]
    fn from(value: SpawnPoint) -> Self {
        let mut this = Self::new();
        this.push(value);
        this
    }
}

/// An actor wants to spawn. This component is also used for respawns.
#[derive(Copy, Clone, Debug, Default)]
pub struct Spawn;

/// A death event.
#[derive(Copy, Clone, Debug, Default)]
pub struct Death;

#[derive(Clone, Debug, Default)]
pub struct ActorProperties {
    /// The rotation that the actor is facing in.
    ///
    /// Defaults to [`IDENTITY`].
    ///
    /// [`IDENTITY`]: Quat::IDENTITY
    pub rotation: Quat,
    /// The local offset (from the actor root) at which the camera sits.
    pub eyes: Vec3,
    // TODO: Add custom props
}
