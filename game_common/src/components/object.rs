use std::time::{Duration, Instant};

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bytemuck::{Pod, Zeroable};

use crate::record::RecordReference;

/// A unique identifer for an object.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct ObjectId(pub RecordReference);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct Object {
    pub id: ObjectId,
}

#[derive(Clone, Default, Debug, Component)]
pub struct ObjectChildren {
    pub children: Vec<Entity>,
}

/// An [`Object`] with a limited lifetime. It will be despawned once expired.
#[derive(Copy, Clone, Debug, Component)]
pub struct Lifetime {
    /// The lifetime of the object.
    pub lifetime: Duration,
    /// The time when the object was spawned. The lifetime will start counting at this time.
    pub start: Instant,
}

impl Lifetime {
    /// Creates a new `Lifetime` with the given [`Duration`].
    #[inline]
    pub fn new(lifetime: Duration) -> Self {
        Self {
            lifetime,
            start: Instant::now(),
        }
    }

    /// Returns `true` if the lifetime is expired.
    #[inline]
    pub fn is_expired(self) -> bool {
        self.start.elapsed() >= self.lifetime
    }
}

/// An [`Object`] that currently being loaded.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct LoadObject {
    pub id: ObjectId,
}

impl LoadObject {
    #[inline]
    pub const fn new(id: ObjectId) -> Self {
        Self { id }
    }
}
