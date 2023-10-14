use glam::{Quat, Vec3};
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::components::inventory::InventorySlotId;
use crate::components::items::ItemId;
use crate::entity::EntityId;

use super::entity::Entity;
use super::source::StreamingSource;

/// A temporary identifier for a snapshot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SnapshotId(pub u32);

impl Add<u32> for SnapshotId {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl AddAssign<u32> for SnapshotId {
    #[inline]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

impl Sub<u32> for SnapshotId {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl SubAssign<u32> for SnapshotId {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

#[derive(Clone, Debug)]
pub enum EntityChange {
    Create {
        entity: Entity,
    },
    Translate {
        id: EntityId,
        translation: Vec3,
    },
    Rotate {
        id: EntityId,
        rotation: Quat,
    },
    // Update { id: EntityId, data: Entity },
    Destroy {
        id: EntityId,
    },
    CreateHost {
        id: EntityId,
    },
    DestroyHost {
        id: EntityId,
    },
    InventoryItemAdd(InventoryItemAdd),
    InventoryItemRemove(InventoryItemRemove),
    InventoryDestroy(InventoryDestroy),
    CreateStreamingSource {
        id: EntityId,
        source: StreamingSource,
    },
    RemoveStreamingSource {
        id: EntityId,
    },
}

impl EntityChange {
    pub const fn entity(&self) -> EntityId {
        match self {
            Self::Create { entity } => entity.id,
            Self::Destroy { id } => *id,
            Self::Translate { id, translation: _ } => *id,
            Self::Rotate { id, rotation: _ } => *id,
            Self::CreateHost { id } => *id,
            Self::DestroyHost { id } => *id,
            Self::InventoryItemAdd(event) => event.entity,
            Self::InventoryItemRemove(event) => event.entity,
            Self::InventoryDestroy(event) => event.entity,
            Self::CreateStreamingSource { id, source: _ } => *id,
            Self::RemoveStreamingSource { id } => *id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InventoryItemAdd {
    pub entity: EntityId,
    pub id: InventorySlotId,
    pub item: ItemId,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryItemRemove {
    pub entity: EntityId,
    pub id: InventorySlotId,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryDestroy {
    pub entity: EntityId,
}
