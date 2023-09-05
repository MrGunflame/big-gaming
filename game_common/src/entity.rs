//! Entity

use bytemuck::{Pod, Zeroable};

/// A unique identifier for a object in the world.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct EntityId {
    index: u64,
}

impl EntityId {
    #[inline]
    pub const fn from_raw(index: u64) -> Self {
        Self { index }
    }

    pub const fn into_raw(self) -> u64 {
        self.index
    }

    #[inline]
    pub const fn dangling() -> Self {
        Self::from_raw(u64::MAX)
    }
}
