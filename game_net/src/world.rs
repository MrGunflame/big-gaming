use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use game_common::entity::{Entity, EntityId};

use crate::snapshot::{EntityChange, SnapshotId};

/// The world state at constant time intervals.
pub struct WorldState {
    snapshots: HashMap<SnapshotId, Entities>,
    last: SnapshotId,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            last: SnapshotId(0),
        }
    }

    pub fn get(&self, id: SnapshotId) -> Option<WorldViewRef<'_>> {
        self.snapshots
            .get(&id)
            .map(|entities| WorldViewRef { entities })
    }

    pub fn get_mut(&mut self, id: SnapshotId) -> Option<WorldViewMut<'_>> {
        self.snapshots.get_mut(&id).map(|entities| WorldViewMut {
            entities,
            delta: Vec::new(),
        })
    }

    pub fn insert(&mut self, id: SnapshotId) {
        let entities = self.snapshots.get(&self.last).cloned().unwrap_or(Entities {
            entities: Vec::new(),
        });
        self.snapshots.insert(id, entities);
        self.last = id;
    }
}

pub struct WorldViewRef<'a> {
    entities: &'a Entities,
}

impl<'a> WorldViewRef<'a> {
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.entities.iter().find(|x| x.id == id)
    }
}

pub struct WorldViewMut<'a> {
    entities: &'a mut Entities,
    delta: Vec<EntityChange>,
}

impl<'a> WorldViewMut<'a> {
    pub fn get_mut(&mut self, id: EntityId) -> Option<EntityMut<'_>> {
        self.entities
            .entities
            .iter_mut()
            .find(|x| x.id == id)
            .map(|entity| EntityMut {
                prev: entity.clone(),
                entity,
                delta: &mut self.delta,
            })
    }

    pub fn spawn(&mut self, entity: Entity) {
        self.entities.entities.push(entity.clone());
        self.delta.push(EntityChange::Create {
            id: entity.id,
            data: entity,
        });
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.entities.entities.retain(|i| i.id != id);
        self.delta.push(EntityChange::Destroy { id });
    }
}

pub struct EntityMut<'a> {
    prev: Entity,
    entity: &'a mut Entity,
    delta: &'a mut Vec<EntityChange>,
}

impl<'a> Deref for EntityMut<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl<'a> DerefMut for EntityMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entity
    }
}

impl<'a> Drop for EntityMut<'a> {
    fn drop(&mut self) {
        if self.prev.transform.translation != self.entity.transform.translation {
            self.delta.push(EntityChange::Translate {
                id: self.entity.id,
                translation: self.entity.transform.translation,
            });
        }

        // TODO: Other deltas
    }
}

#[derive(Clone)]
struct Entities {
    entities: Vec<Entity>,
}

impl Entities {}
