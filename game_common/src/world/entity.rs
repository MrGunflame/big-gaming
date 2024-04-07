use glam::{Quat, Vec3};

use crate::components::components::Components;
use crate::components::object::ObjectId;
use crate::components::race::RaceId;
use crate::components::Transform;
use crate::entity::EntityId;

use super::terrain::TerrainMesh;
use super::world::WorldViewMut;
use super::CellId;

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub transform: Transform,
    pub body: EntityBody,
    pub components: Components,
    pub is_host: bool,
    pub angvel: Vec3,
    pub linvel: Vec3,
}

impl Entity {
    pub fn cell(&self) -> CellId {
        match &self.body {
            EntityBody::Terrain(terrain) => terrain.mesh.cell,
            _ => self.transform.translation.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityBody {
    Terrain(Terrain),
    Object(Object),
    Actor(Actor),
}

impl EntityBody {
    pub const fn kind(&self) -> EntityKind {
        match self {
            Self::Terrain(_) => EntityKind::Terrain,
            Self::Object(_) => EntityKind::Object,
            Self::Actor(_) => EntityKind::Actor,
        }
    }

    #[inline]
    pub const fn as_terrain(&self) -> Option<&Terrain> {
        match self {
            Self::Terrain(terrain) => Some(terrain),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_object(&self) -> Option<&Object> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_actor(&self) -> Option<&Actor> {
        match self {
            Self::Actor(actor) => Some(actor),
            _ => None,
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

impl EntityKind {
    #[inline]
    pub const fn is_terrain(self) -> bool {
        matches!(self, Self::Terrain)
    }

    #[inline]
    pub const fn is_object(self) -> bool {
        matches!(self, Self::Object)
    }

    #[inline]
    pub const fn is_actor(self) -> bool {
        matches!(self, Self::Actor)
    }

    #[inline]
    pub const fn is_item(self) -> bool {
        matches!(self, Self::Item)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Terrain {
    pub mesh: TerrainMesh,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub id: ObjectId,
}

impl Object {
    // pub fn builder() -> ObjectBuilder {
    //     ObjectBuilder::new()
    // }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub race: RaceId,
}

impl From<Terrain> for EntityBody {
    fn from(value: Terrain) -> Self {
        Self::Terrain(value)
    }
}

impl From<Object> for EntityBody {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}

impl From<Actor> for EntityBody {
    fn from(value: Actor) -> Self {
        Self::Actor(value)
    }
}

pub trait BuildEntity {
    fn build(self, view: &mut WorldViewMut<'_>);
}

impl BuildEntity for Entity {
    fn build(self, view: &mut WorldViewMut<'_>) {
        view.spawn(self);
    }
}

/// A queue of [`Entities`] that are ready to be spawned.
///
/// [`Entities`]: Entity
#[derive(Clone, Debug)]
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
    // pub fn new() -> Self {
    //     Self {
    //         transform: Transform::default(),
    //         object: Object { id: ObjectId() },
    //     }
    // }

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
            components: Components::new(),
            is_host: false,
            angvel: Vec3::ZERO,
            linvel: Vec3::ZERO,
        }
    }
}

impl From<ObjectBuilder> for Entity {
    #[inline]
    fn from(value: ObjectBuilder) -> Self {
        value.build()
    }
}

impl From<Terrain> for Entity {
    fn from(value: Terrain) -> Self {
        Entity {
            id: EntityId::dangling(),
            transform: Transform::from_translation(value.mesh.cell.min()),
            body: EntityBody::Terrain(value),
            components: Components::new(),
            is_host: false,
            angvel: Vec3::ZERO,
            linvel: Vec3::ZERO,
        }
    }
}
