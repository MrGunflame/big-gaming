use alloc::vec::Vec;

use crate::components::builtin::CHILDREN;
use crate::components::Component;
use crate::encoding::{Decode, DecodeError, Encode, Reader, Writer};
use crate::entity::EntityId;
use crate::record::RecordReference;

#[derive(Clone, Debug, Default)]
pub struct Children {
    entities: Vec<EntityId>,
}

impl Children {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn insert(&mut self, entity: EntityId) {
        if !self.entities.contains(&entity) {
            self.entities.push(entity);
        }
    }

    pub fn remove(&mut self, entity: EntityId) {
        self.entities.retain(|id| *id != entity);
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity)
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }

    pub fn get(&self) -> &[EntityId] {
        &self.entities
    }
}

impl Encode for Children {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        for entity in &self.entities {
            entity.encode(&mut writer);
        }
    }
}

impl Decode for Children {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let mut entities = Vec::new();
        while !reader.chunk().is_empty() {
            entities.push(EntityId::decode(&mut reader)?);
        }

        Ok(Self { entities })
    }
}

impl Component for Children {
    const ID: RecordReference = CHILDREN;
}
