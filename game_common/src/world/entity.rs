use bevy_ecs::system::{Commands, Resource};
use bevy_scene::SceneBundle;
use bevy_transform::components::Transform;
use bevy_transform::TransformBundle;
use glam::{Quat, Vec3};

use crate::archive::GameArchive;
use crate::bundles::VisibilityBundle;
use crate::components::items::ItemId;
use crate::components::object::{self, LoadObject, ObjectId};

#[derive(Clone, Debug)]
pub enum Entity {
    Object(Object),
    Actor(Actor),
    Item(Item),
}

#[derive(Clone, Debug)]
pub struct Object {
    id: ObjectId,
    transform: Transform,
}

impl Object {
    pub fn builder() -> ObjectBuilder {
        ObjectBuilder::new()
    }
}

#[derive(Clone, Debug)]
pub struct Actor {
    id: u32,
    transform: Transform,
}

#[derive(Clone, Debug)]
pub struct Item {
    id: ItemId,
    transform: Transform,
}

impl From<Object> for Entity {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}

impl From<Item> for Entity {
    fn from(value: Item) -> Self {
        Self::Item(value)
    }
}

impl From<Actor> for Entity {
    fn from(value: Actor) -> Self {
        Self::Actor(value)
    }
}

pub trait BuildEntity {
    fn build(self, archive: &GameArchive, commands: &mut Commands);
}

impl BuildEntity for Object {
    fn build(self, archive: &GameArchive, commands: &mut Commands) {
        let object = archive.objects().get(self.id).unwrap();

        commands
            .spawn(LoadObject::new(self.id))
            .insert(TransformBundle {
                local: self.transform,
                global: Default::default(),
            })
            .insert(VisibilityBundle::new());
    }
}

impl BuildEntity for Entity {
    fn build(self, archive: &GameArchive, commands: &mut Commands) {
        match self {
            Self::Object(object) => object.build(archive, commands),
            _ => todo!(),
        }
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
    pub id: ObjectId,
    pub transform: Transform,
}

impl ObjectBuilder {
    pub fn new() -> Self {
        Self {
            id: ObjectId(0.into()),
            transform: Transform::IDENTITY,
        }
    }

    pub fn id(mut self, id: ObjectId) -> Self {
        self.id = id;
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

    pub fn build(self) -> Object {
        Object {
            id: self.id,
            transform: self.transform,
        }
    }
}

impl From<ObjectBuilder> for Object {
    #[inline]
    fn from(value: ObjectBuilder) -> Self {
        value.build()
    }
}
