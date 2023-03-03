use std::any::Any;
use std::borrow::Borrow;

use ahash::{HashMap, HashMapExt};
use glam::{UVec3, Vec3, Vec3A};

use crate::ecs::components::DynamicComponent;

use super::entity::{Entity, EntityQueue};

pub const CELL_SIZE: Vec3 = Vec3::new(64.0, 64.0, 64.0);
pub const CELL_SIZE_UINT: UVec3 = UVec3::new(64, 64, 64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CellId(u128);

impl CellId {
    // FIXME: What happens if (x|y|z) == CELL_SIZE is currently not
    // well defined. This should properly specified.

    const MASK_X: u128 = 0x0000_0000_FFFF_FFFF__0000_0000_0000_0000;
    const MASK_Y: u128 = 0x0000_0000_0000_0000__FFFF_FFFF_0000_0000;
    const MASK_Z: u128 = 0x0000_0000_0000_0000__0000_0000_FFFF_FFFF;

    /// Creates a new `ChunkId` from the given coordinates.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        // Relative offset based on CHUNK_SIZE.

        let x = if x.is_sign_positive() {
            (x / CELL_SIZE.x) as i32
        } else {
            ((x - CELL_SIZE.x) / CELL_SIZE.x) as i32
        };

        let y = if y.is_sign_positive() {
            (y / CELL_SIZE.y) as i32
        } else {
            ((y - CELL_SIZE.y) / CELL_SIZE.y) as i32
        };

        let z = if z.is_sign_positive() {
            (z / CELL_SIZE.z) as i32
        } else {
            ((z - CELL_SIZE.z) / CELL_SIZE.z) as i32
        };

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
    pub fn min_x(self) -> f32 {
        let x = ((self.0 & Self::MASK_X) >> 64) as i32;
        x as f32 * CELL_SIZE.x
    }

    #[inline]
    pub fn max_x(self) -> f32 {
        self.min_x() + CELL_SIZE.x
    }

    /// Returns the `y` coordinate at which this `ChunkId` starts.
    #[inline]
    pub fn min_y(self) -> f32 {
        let y = ((self.0 & Self::MASK_Y) >> 32) as i32;
        y as f32 * CELL_SIZE.y
    }

    #[inline]
    pub fn max_y(self) -> f32 {
        self.min_y() + CELL_SIZE.y
    }

    /// Returns the `z` coordinate at which this `ChunkId` starts.
    #[inline]
    pub fn min_z(self) -> f32 {
        let z = (self.0 & Self::MASK_Z) as i32;
        z as f32 * CELL_SIZE.z
    }

    #[inline]
    pub fn max_z(self) -> f32 {
        self.min_z() + CELL_SIZE.z
    }

    #[inline]
    pub fn min(self) -> Vec3 {
        Vec3::new(self.min_x(), self.min_y(), self.min_z())
    }

    #[inline]
    pub fn max(self) -> Vec3 {
        Vec3::new(self.max_x(), self.max_y(), self.max_z())
    }
}

impl From<Vec3> for CellId {
    #[inline]
    fn from(value: Vec3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl From<Vec3A> for CellId {
    #[inline]
    fn from(value: Vec3A) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

#[derive(Debug)]
pub struct Cell {
    pub id: CellId,
    next_id: u32,
    entities: HashMap<EntityId, Entity>,
}

impl Cell {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<CellId>,
    {
        Self {
            id: id.into(),
            next_id: 0,
            entities: HashMap::new(),
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

    pub fn spawn<T>(&mut self, entity: T) -> EntityId
    where
        T: Into<Entity>,
    {
        let id = EntityId(self.next_id);
        self.next_id += 1;

        self.entities.insert(id, entity.into());
        id
    }

    pub fn remove<T>(&mut self, id: EntityId)
    where
        T: Borrow<EntityId>,
    {
        self.entities.remove(&id);
    }

    pub fn queue(&self) -> EntityQueue {
        let mut queue = EntityQueue::new();
        queue.extend(self.entities.values().cloned());
        queue
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u32);

#[cfg(test)]
mod tests {
    use super::CellId;

    #[test]
    fn chunk_id() {
        let id = CellId::new(0.0, 0.0, 0.0);
        assert_eq!(id.0, 0);
        assert_eq!(id.min_x(), 0.0);
        assert_eq!(id.min_y(), 0.0);
        assert_eq!(id.min_z(), 0.0);

        let id = CellId::new(128.0, 128.0, 128.0);
        assert_eq!(id.0, (2 << 64) + (2 << 32) + 2);
        assert_eq!(id.min_x(), 128.0);
        assert_eq!(id.min_y(), 128.0);
        assert_eq!(id.min_z(), 128.0);

        let id = CellId::new(156.0, 128.0, 191.0);
        assert_eq!(id.0, (2 << 64) + (2 << 32) + 2);
        assert_eq!(id.min_x(), 128.0);
        assert_eq!(id.min_y(), 128.0);
        assert_eq!(id.min_z(), 128.0);

        let id = CellId::new(1472.0, 36288.0, 48384.0);
        assert_eq!(id.0, (23 << 64) + (567 << 32) + 756);
        assert_eq!(id.min_x(), 1472.0);
        assert_eq!(id.min_y(), 36288.0);
        assert_eq!(id.min_z(), 48384.0);

        let id = CellId::new(-32.0, 0.0, 0.0);
        assert_eq!(id.min_x(), -64.0);
        assert_eq!(id.min_y(), 0.0);
        assert_eq!(id.min_z(), 0.0);

        let id = CellId::new(-63.0, 0.0, -65.0);
        assert_eq!(id.min_x(), -64.0);
        assert_eq!(id.min_y(), 0.0);
        assert_eq!(id.min_z(), -128.0);
    }
}
