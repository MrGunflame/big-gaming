use core::fmt::{self, Debug, Formatter};

use glam::{IVec3, UVec3, Vec3, Vec3A};

use crate::encoding::{Decode, DecodeError, Encode, Reader, Writer};

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
    pub const ZERO: Self = Self::from_i32(IVec3::splat(0));

    const MASK_X: u128 = 0x0000_0000_FFFF_FFFF_0000_0000_0000_0000;
    const MASK_Y: u128 = 0x0000_0000_0000_0000_FFFF_FFFF_0000_0000;
    const MASK_Z: u128 = 0x0000_0000_0000_0000_0000_0000_FFFF_FFFF;

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
    pub const fn from_i32(vec: IVec3) -> Self {
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

impl Encode for CellId {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        let (x, y, z) = self.as_parts();
        x.encode(&mut writer);
        y.encode(&mut writer);
        z.encode(&mut writer);
    }
}

impl Decode for CellId {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let x = u32::decode(&mut reader)?;
        let y = u32::decode(&mut reader)?;
        let z = u32::decode(&mut reader)?;
        Ok(Self::from_parts(x, y, z))
    }
}
