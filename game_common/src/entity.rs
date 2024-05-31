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

impl From<EntityId> for game_wasm::entity::EntityId {
    fn from(value: EntityId) -> Self {
        Self::from_raw(value.into_raw())
    }
}

impl From<game_wasm::entity::EntityId> for EntityId {
    fn from(value: game_wasm::entity::EntityId) -> Self {
        Self::from_raw(value.into_raw())
    }
}
