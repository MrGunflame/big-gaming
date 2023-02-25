//! Entity

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ahash::HashMap;
use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;
use bevy_transform::prelude::Transform;
use parking_lot::RwLock;

use crate::components::object::ObjectId;

static ENTITY_ID: AtomicU64 = AtomicU64::new(0);

/// A unique identifier for a object in the world.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityId {
    index: u64,
}

impl EntityId {
    pub fn new() -> Self {
        let id = ENTITY_ID.fetch_add(1, Ordering::Relaxed);
        Self { index: id }
    }
}

#[derive(Clone, Debug, Component, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub transform: Transform,
    pub data: EntityData,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityData {
    Object { id: ObjectId },
    Actor {},
}

impl EntityData {
    pub const fn is_object(&self) -> bool {
        matches!(self, Self::Object { id: _ })
    }

    pub const fn is_actor(&self) -> bool {
        matches!(self, Self::Actor {})
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
