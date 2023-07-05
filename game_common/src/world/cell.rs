use std::fmt::{self, Debug, Formatter};

use glam::{IVec3, UVec3, Vec3, Vec3A};

use super::entity::Entity;
use super::world::{CellViewRef, WorldViewMut};

pub const CELL_SIZE: Vec3 = Vec3::new(64.0, 64.0, 64.0);
pub const CELL_SIZE_UINT: UVec3 = UVec3::new(64, 64, 64);

/// A unique identfier for a cell.
///
/// Note that a cell ranges from `CELL_SIZE.(x|y|z) <= (x|y|z) > CELL_SIZE.(x|y|z)`, i.e. a new
/// cell starts at exactly the multiplier of `CELL_SIZE.x`.
///
/// For example, with a cell size of 64, a cell ranges from `(0.0, 0.0, 0.0)` to
/// `(63.9999, 0.0, 0.0)`, but `(64.0, 0.0, 0.0)` is the new cell.
///
/// For negative coordinates the direction is still directed into the positive range.
///
///
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CellId(u128);

impl CellId {
    const MASK_X: u128 = 0x0000_0000_FFFF_FFFF__0000_0000_0000_0000;
    const MASK_Y: u128 = 0x0000_0000_0000_0000__FFFF_FFFF_0000_0000;
    const MASK_Z: u128 = 0x0000_0000_0000_0000__0000_0000_FFFF_FFFF;

    /// Creates a new `CellId` from the given coordinates.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        // This is the same as (x / CELL_SIZE.x) as i32 - (x.to_bits() >> 31) as i32,
        // but results in the same assembly.

        let x = if x.is_sign_negative() {
            (x / CELL_SIZE.x) as i32 - 1
        } else {
            (x / CELL_SIZE.x) as i32
        };

        let y = if y.is_sign_negative() {
            (y / CELL_SIZE.y) as i32 - 1
        } else {
            (y / CELL_SIZE.y) as i32
        };

        let z = if z.is_sign_negative() {
            (z / CELL_SIZE.z) as i32 - 1
        } else {
            (z / CELL_SIZE.z) as i32
        };

        Self::from_i32(IVec3::new(x, y, z))
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

    #[inline]
    pub fn to_i32(self) -> IVec3 {
        let x = ((self.0 & Self::MASK_X) >> 64) as i32;
        let y = ((self.0 & Self::MASK_Y) >> 32) as i32;
        let z = (self.0 & Self::MASK_Z) as i32;
        IVec3::new(x, y, z)
    }

    #[inline]
    pub fn from_i32(vec: IVec3) -> Self {
        Self::from_parts(vec.x as u32, vec.y as u32, vec.z as u32)
    }

    /// Returns a `f32` representation of the `CellId`.
    #[inline]
    pub fn to_f32(self) -> Vec3 {
        let x = (((self.0 & Self::MASK_X) >> 64) as i32) as f32;
        let y = (((self.0 & Self::MASK_Y) >> 32) as i32) as f32;
        let z = ((self.0 & Self::MASK_Z) as i32) as f32;
        Vec3::new(x, y, z)
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

impl Debug for CellId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CellId").field(&self.to_i32()).finish()
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
    id: CellId,
    entities: Vec<Entity>,
    #[cfg(debug_assertions)]
    loaded: bool,
}

impl Cell {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<CellId>,
    {
        Self {
            id: id.into(),
            entities: Vec::new(),
            #[cfg(debug_assertions)]
            loaded: false,
        }
    }

    pub fn id(&self) -> CellId {
        self.id
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    pub fn spawn<T>(&mut self, entity: T)
    where
        T: Into<Entity>,
    {
        self.entities.push(entity.into());
    }

    pub fn load(&mut self, view: &mut WorldViewMut<'_>) {
        #[cfg(debug_assertions)]
        {
            self.loaded = true;
        }

        for entity in &mut self.entities {
            entity.id = view.spawn(entity.clone());
        }
    }

    pub fn unload(&mut self, view: &mut WorldViewMut<'_>) {
        #[cfg(debug_assertions)]
        assert!(self.loaded, "attempted to unload cell before it was loaded");

        for entity in &self.entities {
            view.despawn(entity.id);
        }
    }

    pub fn update(&mut self, view: &CellViewRef<'_>) {
        #[cfg(debug_assertions)]
        assert!(self.loaded, "attempted to update cell before it was loaded");

        // Expect a similar amount of entities.
        let mut entities = Vec::with_capacity(self.entities.len());

        for entity in view.iter() {
            entities.push(entity.clone());
        }
    }
}

/// Returns all cells around center.
pub fn square(center: CellId, distance: u32) -> Vec<CellId> {
    debug_assert!(distance as i32 >= 0);

    let distance = distance as i32;
    let mut cells = vec![];

    let center = center.to_i32();
    for x in center.x - distance..=center.x + distance {
        for y in center.y - distance..=center.y + distance {
            for z in center.z - distance..=center.z + distance {
                cells.push(CellId::from_i32(IVec3::new(
                    center.x + x,
                    center.y + y,
                    center.z + z,
                )));
            }
        }
    }

    cells
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u32);

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use glam::{IVec3, Vec3};

    use super::{square, CellId, CELL_SIZE};

    #[test]
    fn cell_size_min() {
        // This is the smallest possible CELL_SIZE that is acceptable.
        // Smaller sizes will break several implementations and tests.
        assert!(CELL_SIZE.x >= 3.0);
        assert!(CELL_SIZE.y >= 3.0);
        assert!(CELL_SIZE.z >= 3.0);
    }

    #[test]
    fn cell_id_negative() {
        let id = CellId::new(-64.0, -128.0, -63.99);

        assert_eq!(id, CellId::from_i32(IVec3::new(-2, -3, -1)));
    }

    #[test]
    fn cell_to_i32_zero() {
        let id = CellId::new(0.0, 0.0, 0.0);

        let vec = id.to_i32();

        assert_eq!(vec.x, 0);
        assert_eq!(vec.y, 0);
        assert_eq!(vec.z, 0);
    }

    #[test]
    fn cell_to_i32_positive() {
        let id = CellId::new(CELL_SIZE.x * 3.0, 0.0, 0.0);

        let vec = id.to_i32();

        assert_eq!(vec.x, 3);
        assert_eq!(vec.y, 0);
        assert_eq!(vec.z, 0);
    }

    #[test]
    fn cell_to_i32_negative() {
        let id = CellId::new(CELL_SIZE.x * -3.0, 0.0, 0.0);

        let vec = id.to_i32();

        assert_eq!(vec.x, -4);
        assert_eq!(vec.y, 0);
        assert_eq!(vec.z, 0);
    }

    #[test]
    fn from_i32_zero() {
        let vec = IVec3::new(0, 0, 0);
        let id = CellId::new(0.0, 0.0, 0.0);

        assert_eq!(CellId::from_i32(vec), id);
    }

    #[test]
    fn from_i32_positive() {
        let vec = IVec3::new(1, 2, 3);
        let id = CellId::new(CELL_SIZE.x * 1.0, CELL_SIZE.y * 2.0, CELL_SIZE.z * 3.0);

        assert_eq!(CellId::from_i32(vec), id);
    }

    #[test]
    fn from_i32_negative() {
        let vec = IVec3::new(-3, -2, -1);
        let id = CellId::new(CELL_SIZE.x * -2.0, CELL_SIZE.y * -1.0, -0.0);

        assert_eq!(CellId::from_i32(vec), id);
    }

    #[test]
    fn cell_id_min_zero() {
        let id = CellId::new(0.0, 0.0, 0.0);
        assert_eq!(id.min(), Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn cell_id_min_positive() {
        let id = CellId::new(32.0, 64.0, 127.0);
        assert_eq!(id.min(), Vec3::new(0.0, 64.0, 64.0));
    }

    #[test]
    fn cell_id_min_negative() {
        let id = CellId::new(-0.0, -32.0, -64.0);
        assert_eq!(id.min(), Vec3::new(-64.0, -64.0, -128.0));
    }

    #[test]
    fn cell_id_square_0() {
        let center = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 0;

        let res = square(center, distance);
        match_cells_exact(&res, &[center]);
    }

    #[test]
    fn cell_id_square_1() {
        let center = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 1;

        let res = square(center, distance);
        match_cells_exact(
            &res,
            &[
                center,
                CellId::from_i32(IVec3::new(0, 0, 1)),
                CellId::from_i32(IVec3::new(0, 0, -1)),
                CellId::from_i32(IVec3::new(0, 1, 0)),
                CellId::from_i32(IVec3::new(0, 1, 1)),
                CellId::from_i32(IVec3::new(0, 1, -1)),
                CellId::from_i32(IVec3::new(0, -1, 0)),
                CellId::from_i32(IVec3::new(0, -1, 1)),
                CellId::from_i32(IVec3::new(0, -1, -1)),
                CellId::from_i32(IVec3::new(1, 0, 0)),
                CellId::from_i32(IVec3::new(1, 0, 1)),
                CellId::from_i32(IVec3::new(1, 0, -1)),
                CellId::from_i32(IVec3::new(1, 1, 0)),
                CellId::from_i32(IVec3::new(1, 1, 1)),
                CellId::from_i32(IVec3::new(1, 1, -1)),
                CellId::from_i32(IVec3::new(1, -1, 0)),
                CellId::from_i32(IVec3::new(1, -1, 1)),
                CellId::from_i32(IVec3::new(1, -1, -1)),
                CellId::from_i32(IVec3::new(-1, 0, 0)),
                CellId::from_i32(IVec3::new(-1, 0, 1)),
                CellId::from_i32(IVec3::new(-1, 0, -1)),
                CellId::from_i32(IVec3::new(-1, 1, 0)),
                CellId::from_i32(IVec3::new(-1, 1, 1)),
                CellId::from_i32(IVec3::new(-1, 1, -1)),
                CellId::from_i32(IVec3::new(-1, -1, 0)),
                CellId::from_i32(IVec3::new(-1, -1, 1)),
                CellId::from_i32(IVec3::new(-1, -1, -1)),
            ],
        );
    }

    #[track_caller]
    fn match_cells_exact(lhs: &[CellId], rhs: &[CellId]) {
        let mut lhs: HashSet<_> = lhs.iter().copied().collect();

        for id in rhs {
            if !lhs.remove(&id) {
                panic!("missing {:?} in lhs", id);
            }
        }

        for id in lhs {
            panic!("missing {:?} in rhs", id);
        }
    }
}
