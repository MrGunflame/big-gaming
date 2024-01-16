use bytemuck::{Pod, Zeroable};

use crate::components::{Decode, Encode};

/// A unique identifier for an [`Entity`].
///
/// [`Entity`]: crate::world::Entity
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod, Encode, Decode)]
#[repr(transparent)]
pub struct EntityId(u64);

impl EntityId {
    /// Creates a `EntityId` using the specified `bits`.
    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns the underlying bits of the `EntityId`.
    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> &u64 {
        &self.0
    }
}
