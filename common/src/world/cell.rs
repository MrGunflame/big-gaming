use ahash::{HashSet, HashSetExt};
use bevy_ecs::prelude::Entity;
use glam::Vec3;

pub const CELL_SIZE: Vec3 = Vec3::new(64.0, 64.0, 64.0);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CellId(u128);

impl CellId {
    const MASK_X: u128 = 0x0000_0000_FFFF_FFFF__0000_0000_0000_0000;
    const MASK_Y: u128 = 0x0000_0000_0000_0000__FFFF_FFFF_0000_0000;
    const MASK_Z: u128 = 0x0000_0000_0000_0000__0000_0000_FFFF_FFFF;

    /// Creates a new `ChunkId` from the given coordinates.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        // Relative offset based on CHUNK_SIZE.
        let x = (x / CELL_SIZE.x) as i32;
        let y = (y / CELL_SIZE.y) as i32;
        let z = (z / CELL_SIZE.z) as i32;

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
        x as f32 * CELL_SIZE.x
    }

    /// Returns the `y` coordinate at which this `ChunkId` starts.
    #[inline]
    pub fn y(self) -> f32 {
        let y = (self.0 & Self::MASK_Y) >> 32;
        y as f32 * CELL_SIZE.y
    }

    /// Returns the `z` coordinate at which this `ChunkId` starts.
    #[inline]
    pub fn z(self) -> f32 {
        let z = self.0 & Self::MASK_Z;
        z as f32 * CELL_SIZE.z
    }
}

pub struct Cell {
    pub id: CellId,
    entities: HashSet<Entity>,
}

impl Cell {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<CellId>,
    {
        Self {
            id: id.into(),
            entities: HashSet::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity);
    }

    pub fn remove(&mut self, entity: Entity) -> Option<Entity> {
        if self.entities.remove(&entity) {
            Some(entity)
        } else {
            None
        }
    }
}
