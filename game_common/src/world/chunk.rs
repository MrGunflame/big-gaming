//! Splitting the world into many chunks.
//!
//! A [`Chunk`] is a single *cell* in the game world. This chunking is necessary to only load
//! cells that need to be active, and save resources by exluding [`Chunk`]s that don't need to be
//! loaded.
//!
//!

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use glam::{Vec3, Vec3A};

/// The size of a chunk.
pub const CHUNK_SIZE: Vec3 = Vec3::new(64.0, 64.0, 64.0);

/// A unique identifier for a [`Chunk`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkId(u128);

impl ChunkId {
    const MASK_X: u128 = 0x0000_0000_FFFF_FFFF__0000_0000_0000_0000;
    const MASK_Y: u128 = 0x0000_0000_0000_0000__FFFF_FFFF_0000_0000;
    const MASK_Z: u128 = 0x0000_0000_0000_0000__0000_0000_FFFF_FFFF;

    /// Creates a new `ChunkId` from the given coordinates.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        // Relative offset based on CHUNK_SIZE.
        let x = (x / CHUNK_SIZE.x) as i32;
        let y = (y / CHUNK_SIZE.y) as i32;
        let z = (z / CHUNK_SIZE.z) as i32;

        Self::from_parts(x as u32, y as u32, z as u32)
    }

    pub const fn as_parts(self) -> (u32, u32, u32) {
        (
            ((self.0 & Self::MASK_X) >> 64) as u32,
            ((self.0 & Self::MASK_Y) >> 32) as u32,
            (self.0 & Self::MASK_Z) as u32,
        )
    }

    #[inline]
    pub const fn from_parts(x: u32, y: u32, z: u32) -> Self {
        let x = (x as u128) << 64;
        let y = (y as u128) << 32;
        let z = z as u128;

        Self(x | y | z)
    }

    /// Returns the `x` coordinate at which this `ChunkId` starts.
    ///
    /// The resulting chunk will span `x() + CHUNK_SIZE.x`.
    #[inline]
    pub fn x(self) -> f32 {
        let x = (self.0 & Self::MASK_X) >> 64;
        x as f32 * CHUNK_SIZE.x
    }

    /// Returns the `y` coordinate at which this `ChunkId` starts.
    #[inline]
    pub fn y(self) -> f32 {
        let y = (self.0 & Self::MASK_Y) >> 32;
        y as f32 * CHUNK_SIZE.y
    }

    /// Returns the `z` coordinate at which this `ChunkId` starts.
    #[inline]
    pub fn z(self) -> f32 {
        let z = self.0 & Self::MASK_Z;
        z as f32 * CHUNK_SIZE.z
    }

    /// Creates a [`ChunkIdRange`] over all adjacent `ChunkId`s in the given `radius`. The
    /// returned range includes the current `ChunkId`.
    pub fn radius(self, radius: u32) -> Radius {
        Radius {
            center: self,
            radius,
            x: 0,
            y: 0,
            z: 0,
        }

        // ChunkIdRange {
        //     x: x - radius,
        //     x_end: x + radius,
        //     y: y - radius,
        //     y_end: y + radius,
        //     z: z - radius,
        //     z_end: z + radius,
        // }
    }
}

impl Display for ChunkId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<Vec3> for ChunkId {
    #[inline]
    fn from(value: Vec3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl From<Vec3A> for ChunkId {
    #[inline]
    fn from(value: Vec3A) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

/// An `Iterator` of [`ChunkId`]s.
///
/// Note that the returned order is unspecified.
#[derive(Copy, Clone, Debug)]
pub struct Radius {
    center: ChunkId,
    radius: u32,
    x: u32,
    y: u32,
    z: u32,
}

impl Iterator for Radius {
    type Item = ChunkId;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for Radius {
    #[inline]
    fn len(&self) -> usize {
        let factor = (self.radius * 2 + 1) as usize;
        factor * factor * factor
    }
}

/// A loaded chunk of the game world.
// TODO: A Vec is not a good collection for entities.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// A unique identifer for this `Chunk`.
    pub id: ChunkId,
    /// The entities that currently reside in this `Chunk`.
    entities: Vec<Entity>,
}

impl Chunk {
    /// Creates a new, empty `Chunk`.
    #[inline]
    pub fn new<T>(id: T) -> Self
    where
        T: Into<ChunkId>,
    {
        Self {
            id: id.into(),
            entities: Vec::new(),
        }
    }

    /// Returns the number of entities in this `Chunk`.
    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` if this `Chunk` is empty, i.e. has no entities.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if this `Chunk` contains the given [`Entity`].
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    /// Removes all entities from this `Chunk`.
    #[inline]
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    /// Inserts a new [`Entity`] into this `Chunk`.
    pub fn insert(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    /// Removes and returns an [`Entity`] from this `Chunk`.
    pub fn remove(&mut self, entity: Entity) -> Option<Entity> {
        let index = self
            .entities
            .iter()
            .enumerate()
            .find(|(_, e)| **e == entity)
            .map(|(i, _)| i)?;

        Some(self.entities.swap_remove(index))
    }
}

/// A registry for [`Chunk`]s.
#[derive(Clone, Debug, Resource)]
pub struct ChunkRegistry {
    active: HashMap<ChunkId, Chunk>,
}

impl ChunkRegistry {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
        }
    }

    /// Returns the [`Chunk`] with the given [`ChunkId`].
    ///
    /// If the requested chunk is not loaded currently, it is loaded.
    pub fn get<T>(&mut self, id: T) -> &mut Chunk
    where
        T: Borrow<ChunkId>,
    {
        if !self.active.contains_key(id.borrow()) {
            self.active.insert(*id.borrow(), Chunk::new(*id.borrow()));
        }

        self.active.get_mut(id.borrow()).unwrap()
    }

    /// Requests a [`Chunk`] to be loaded in the background. Returns `true` if the requested
    /// [`Chunk`] is ready.
    pub fn load<T>(&self, id: T) -> bool
    where
        T: Borrow<ChunkId>,
    {
        if self.active.contains_key(id.borrow()) {
            return true;
        }

        tracing::trace!("Loading chunk {}", id.borrow());
        false
    }
}

/// A delta of entities that have become active or inactive since the last chunk tick.
#[derive(Clone, Debug, Default)]
pub struct ChunkTransition {
    pub activated: Vec<Entity>,
    pub deactivated: Vec<Entity>,
}

impl ChunkTransition {
    pub fn apply(&self, world: &mut World) {
        for entity in &self.deactivated {
            tracing::trace!("Despawning {:?}", entity);
            world.entity_mut(*entity).despawn();
        }

        for entity in &self.activated {
            tracing::trace!("Spawning new entity");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkId;

    #[test]
    fn chunk_id() {
        let id = ChunkId::new(0.0, 0.0, 0.0);
        assert_eq!(id.0, 0);
        assert_eq!(id.x(), 0.0);
        assert_eq!(id.y(), 0.0);
        assert_eq!(id.z(), 0.0);

        let id = ChunkId::new(128.0, 128.0, 128.0);
        assert_eq!(id.0, (2 << 64) + (2 << 32) + 2);
        assert_eq!(id.x(), 128.0);
        assert_eq!(id.y(), 128.0);
        assert_eq!(id.z(), 128.0);

        let id = ChunkId::new(156.0, 128.0, 191.0);
        assert_eq!(id.0, (2 << 64) + (2 << 32) + 2);
        assert_eq!(id.x(), 128.0);
        assert_eq!(id.y(), 128.0);
        assert_eq!(id.z(), 128.0);

        let id = ChunkId::new(1472.0, 36288.0, 48384.0);
        assert_eq!(id.0, (23 << 64) + (567 << 32) + 756);
        assert_eq!(id.x(), 1472.0);
        assert_eq!(id.y(), 36288.0);
        assert_eq!(id.z(), 48384.0);
    }

    #[test]
    fn chunk_id_radius() {
        let mut iter = ChunkId::new(0.0, 0.0, 0.0).radius(0);
        assert_eq!(iter.len(), 1);

        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 0.0, 0.0)));
        assert_eq!(iter.next(), None);

        let mut iter = ChunkId::new(32.0, 32.0, 32.0).radius(1);
        assert_eq!(iter.len(), 27);

        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 0.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 0.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 0.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 32.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 32.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 32.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 64.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 64.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(0.0, 64.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 0.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 0.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 0.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 32.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 32.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 32.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 64.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 64.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(32.0, 64.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 0.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 0.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 0.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 32.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 32.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 32.0, 64.0)));

        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 64.0, 0.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 64.0, 32.0)));
        assert_eq!(iter.next(), Some(ChunkId::new(64.0, 64.0, 64.0)));

        assert_eq!(iter.next(), None);
    }
}
