use bevy_ecs::system::{Commands, Resource};
use bevy_transform::components::Transform;
use bevy_transform::TransformBundle;
use glam::{Quat, Vec3};

use crate::archive::GameArchive;
use crate::bundles::VisibilityBundle;
use crate::components::items::{ItemId, LoadItem};
use crate::components::object::{LoadObject, ObjectId};
use crate::components::terrain::LoadTerrain;

use super::terrain::TerrainMesh;

#[derive(Clone, Debug, PartialEq)]
pub enum Entity {
    Terrain(TerrainMesh),
    Object(Object),
    Actor(Actor),
    Item(Item),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Terrain {}

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub id: ObjectId,
    pub transform: Transform,
}

impl Object {
    pub fn builder() -> ObjectBuilder {
        ObjectBuilder::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: u32,
    pub transform: Transform,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub transform: Transform,
}

impl From<TerrainMesh> for Entity {
    fn from(value: TerrainMesh) -> Self {
        Self::Terrain(value)
    }
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

impl BuildEntity for TerrainMesh {
    fn build(self, archive: &GameArchive, commands: &mut Commands) {
        let cell = self.cell;

        dbg!(cell);

        commands
            .spawn(LoadTerrain {
                cell: self.cell,
                mesh: self,
            })
            .insert(TransformBundle {
                local: Transform::from_translation(cell.min()),
                global: Default::default(),
            })
            .insert(VisibilityBundle::new());
    }
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

impl BuildEntity for Item {
    fn build(self, archive: &GameArchive, commands: &mut Commands) {
        let item = archive.items().get(self.id).unwrap();

        commands
            .spawn(LoadItem { id: self.id })
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
            Self::Terrain(terrain) => terrain.build(archive, commands),
            Self::Object(object) => object.build(archive, commands),
            Self::Item(item) => item.build(archive, commands),
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
