//! World generation
//!
//!

use game_wasm::components::Component;
use game_wasm::encoding::BinaryWriter;

use crate::components::components::{Components, RawComponent};
use crate::components::Transform;
use crate::record::RecordReference;

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
    pub components: Components,
}

impl EntityBuilder {
    pub fn new() -> Self {
        Self {
            components: Components::new(),
        }
    }

    pub fn transform(mut self, transform: Transform) -> Self {
        let (fields, data) = BinaryWriter::new().encoded(&transform);
        self.components
            .insert(Transform::ID, RawComponent::new(data, fields));
        self
    }

    pub fn component(mut self, id: RecordReference, component: RawComponent) -> Self {
        self.components.insert(id, component);
        self
    }
}

impl Default for EntityBuilder {
    fn default() -> Self {
        Self::new()
    }
}
