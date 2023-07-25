//! Entity

use std::sync::Arc;

use ahash::HashMap;
use bevy_ecs::system::Resource;
use bytemuck::{Pod, Zeroable};
use parking_lot::RwLock;

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

#[derive(Clone, Debug, Default, Resource)]
pub struct EntityMap {
    inner: Arc<RwLock<EntityMapInner>>,
}

#[derive(Clone, Debug, Default)]
struct EntityMapInner {
    inner: HashMap<EntityId, bevy_ecs::entity::Entity>,
    inner2: HashMap<bevy_ecs::entity::Entity, EntityId>,
}

impl EntityMap {
    pub fn insert(&self, id: EntityId, ent: bevy_ecs::entity::Entity) {
        let mut inner = self.inner.write();

        inner.inner.insert(id, ent);
        inner.inner2.insert(ent, id);
    }

    pub fn get(&self, id: EntityId) -> Option<bevy_ecs::entity::Entity> {
        let inner = self.inner.read();

        inner.inner.get(&id).copied()
    }

    pub fn get_entity(&self, ent: bevy_ecs::entity::Entity) -> Option<EntityId> {
        let inner = self.inner.read();

        inner.inner2.get(&ent).copied()
    }
}
