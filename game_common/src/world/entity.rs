use bevy_ecs::prelude::Component;
use bevy_ecs::system::Resource;
use bevy_transform::components::Transform;
use glam::{Quat, Vec3};

use crate::components::combat::Health;
use crate::components::items::ItemId;
use crate::components::object::ObjectId;
use crate::components::race::RaceId;
use crate::entity::EntityId;

use super::terrain::TerrainMesh;
use super::world::WorldViewMut;
use super::CellId;

#[derive(Clone, Debug, Component, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub transform: Transform,
    pub body: EntityBody,
}

impl Entity {
    pub fn cell(&self) -> CellId {
        match &self.body {
            EntityBody::Terrain(terrain) => terrain.cell,
            _ => self.transform.translation.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityBody {
    Terrain(TerrainMesh),
    Object(Object),
    Actor(Actor),
    Item(Item),
}

impl EntityBody {
    pub const fn kind(&self) -> EntityKind {
        match self {
            Self::Terrain(_) => EntityKind::Terrain,
            Self::Object(_) => EntityKind::Object,
            Self::Actor(_) => EntityKind::Actor,
            Self::Item(_) => EntityKind::Item,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Terrain,
    Object,
    Actor,
    Item,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Terrain {}

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub id: ObjectId,
}

impl Object {
    pub fn builder() -> ObjectBuilder {
        ObjectBuilder::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub race: RaceId,
    pub health: Health,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: ItemId,
}

impl From<TerrainMesh> for EntityBody {
    fn from(value: TerrainMesh) -> Self {
        Self::Terrain(value)
    }
}

impl From<Object> for EntityBody {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}

impl From<Item> for EntityBody {
    fn from(value: Item) -> Self {
        Self::Item(value)
    }
}

impl From<Actor> for EntityBody {
    fn from(value: Actor) -> Self {
        Self::Actor(value)
    }
}

pub trait BuildEntity {
    fn build(self, view: &mut WorldViewMut);
}

impl BuildEntity for Entity {
    fn build(self, view: &mut WorldViewMut) {
        view.spawn(self);
    }
}

/// A queue of [`Entities`] that are ready to be spawned.
///
/// [`Entities`]: Entity
#[derive(Clone, Debug, Resource)]
pub struct EntityQueue {
    // Note that we build the entities in the reverse order they were
    // queued. This is more efficient and has no other effects since the
    // queue is always emptied every tick.
    queue: Vec<Entity>,
}

impl EntityQueue {
    pub fn new() -> Self {
        Self {
            // Start with a small preallocation to make initial groth faster.
            queue: Vec::with_capacity(16),
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn push(&mut self, entity: Entity) {
        self.queue.push(entity);
    }

    pub fn pop(&mut self) -> Option<Entity> {
        self.queue.pop()
    }
}

impl<A> Extend<A> for EntityQueue
where
    A: Into<Entity>,
{
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        for entity in iter.into_iter() {
            self.push(entity.into());
        }
    }
}

impl Default for EntityQueue {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for EntityQueue {
    type Item = Entity;
    type IntoIter = std::vec::IntoIter<Entity>;

    fn into_iter(self) -> Self::IntoIter {
        self.queue.into_iter()
    }
}

#[derive(Clone, Debug)]
pub struct ObjectBuilder {
    transform: Transform,
    object: Object,
}

impl ObjectBuilder {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            object: Object {
                id: ObjectId(0.into()),
            },
        }
    }

    pub fn id(mut self, id: ObjectId) -> Self {
        self.object.id = id;
        self
    }

    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn translation(mut self, translation: Vec3) -> Self {
        self.transform.translation = translation;
        self
    }

    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.transform.rotation = rotation;
        self
    }

    pub fn scale(mut self, scale: Vec3) -> Self {
        self.transform.scale = scale;
        self
    }

    pub fn build(self) -> Entity {
        Entity {
            id: EntityId::dangling(),
            transform: self.transform,
            body: EntityBody::Object(self.object),
        }
    }
}

impl From<ObjectBuilder> for Entity {
    #[inline]
    fn from(value: ObjectBuilder) -> Self {
        value.build()
    }
}

impl From<TerrainMesh> for Entity {
    fn from(value: TerrainMesh) -> Self {
        Entity {
            id: EntityId::dangling(),
            transform: Transform::from_translation(value.cell.min()),
            body: EntityBody::Terrain(value),
        }
    }
}
