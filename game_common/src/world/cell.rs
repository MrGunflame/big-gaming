use std::any::Any;
use std::borrow::Borrow;

use ahash::{HashMap, HashMapExt};
use glam::{Vec3, Vec3A};

use crate::ecs::components::DynamicComponent;

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
    entities: HashMap<EntityId, EntityComponents>,
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

    pub fn spawn(&mut self) -> EntityMut<'_> {
        let id = EntityId(self.next_id);
        self.next_id += 1;

        self.entities.insert(
            id,
            EntityComponents {
                id,
                components: Vec::new(),
            },
        );

        EntityMut {
            entity: self.entities.get_mut(&id).unwrap(),
        }
    }

    pub fn remove<T>(&mut self, id: EntityId)
    where
        T: Borrow<EntityId>,
    {
        self.entities.remove(&id);
    }
}

#[derive(Debug)]
pub struct EntityComponents {
    id: EntityId,
    components: Vec<DynamicComponent>,
}

pub struct EntityMut<'a> {
    entity: &'a mut EntityComponents,
}

impl<'a> EntityMut<'a> {
    #[inline]
    pub fn id(&self) -> EntityId {
        self.entity.id
    }

    pub fn insert<T>(&'a mut self, component: T) -> &'a mut Self
    where
        T: Into<DynamicComponent>,
    {
        self.entity.components.push(component.into());
        self
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
        assert_eq!(id.x(), 0.0);
        assert_eq!(id.y(), 0.0);
        assert_eq!(id.z(), 0.0);

        let id = CellId::new(128.0, 128.0, 128.0);
        assert_eq!(id.0, (2 << 64) + (2 << 32) + 2);
        assert_eq!(id.x(), 128.0);
        assert_eq!(id.y(), 128.0);
        assert_eq!(id.z(), 128.0);

        let id = CellId::new(156.0, 128.0, 191.0);
        assert_eq!(id.0, (2 << 64) + (2 << 32) + 2);
        assert_eq!(id.x(), 128.0);
        assert_eq!(id.y(), 128.0);
        assert_eq!(id.z(), 128.0);

        let id = CellId::new(1472.0, 36288.0, 48384.0);
        assert_eq!(id.0, (23 << 64) + (567 << 32) + 756);
        assert_eq!(id.x(), 1472.0);
        assert_eq!(id.y(), 36288.0);
        assert_eq!(id.z(), 48384.0);
    }
}
