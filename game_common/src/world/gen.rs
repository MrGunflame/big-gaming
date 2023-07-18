//! World generation
//!
//!

use glam::{Quat, Vec3};

use crate::components::components::{Component, Components};
use crate::components::transform::Transform;
use crate::record::RecordReference;

use super::entity::Terrain;
use super::CellId;
pub mod flat;

pub struct Generator {
    inner: Box<dyn Generate>,
}

impl Generator {
    pub fn generate(&self, cell: &mut CellBuilder) {
        tracing::info!("Generating cell {:?}", cell.id().as_parts());

        self.inner.generate(cell);
    }
}

pub trait Generate: Send + Sync + 'static {
    /// Generates a [`Cell`] in the level for the first time.
    fn generate(&self, cell: &mut CellBuilder);
}

impl<T> From<T> for Generator
where
    T: Generate,
{
    fn from(value: T) -> Self {
        Generator {
            inner: Box::new(value),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CellBuilder {
    id: CellId,
    entities: Vec<EntityBuilder>,
}

impl CellBuilder {
    pub fn new(id: CellId) -> Self {
        Self {
            id,
            entities: Vec::new(),
        }
    }

    pub fn id(&self) -> CellId {
        self.id
    }

    pub fn spawn(&mut self, entity: EntityBuilder) {
        self.entities.push(entity);
    }

    pub fn into_entities(self) -> Vec<EntityBuilder> {
        self.entities
    }
}

#[derive(Clone, Debug)]
pub struct EntityBuilder {
    pub id: RecordReference,
    pub transform: Transform,
    pub components: Components,
    pub terrain: Option<Terrain>,
}

impl EntityBuilder {
    pub fn new(id: RecordReference) -> Self {
        Self {
            id,
            transform: Transform::default(),
            components: Components::new(),
            terrain: None,
        }
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

    pub fn component(mut self, id: RecordReference, component: Component) -> Self {
        self.components.insert(id, component);
        self
    }

    pub fn terrain(mut self, map: Terrain) -> Self {
        self.terrain = Some(map);
        self
    }
}
