//! Splitting the world into many chunks.
//!
//! A [`Chunk`] is a single *cell* in the game world. This chunking is necessary to only load
//! cells that need to be active, and save resources by exluding [`Chunk`]s that don't need to be
//! loaded.
//!
//!

use std::borrow::Borrow;
use std::collections::HashMap;

use bevy_ecs::entity::Entity;
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
#[derive(Clone, Debug)]
pub struct ChunkHandler {
    active: HashMap<ChunkId, Chunk>,
}

impl ChunkHandler {
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

        false
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
}
