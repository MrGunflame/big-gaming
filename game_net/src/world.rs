use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityId};

#[cfg(feature = "tracing")]
use tracing::{event, span, Level, Span};

use crate::snapshot::{EntityChange, SnapshotId};

/// The world state at constant time intervals.
#[derive(Clone, Debug, Resource)]
pub struct WorldState {
    snapshots: HashMap<SnapshotId, Snapshot>,
    last: SnapshotId,
    delta: Vec<EntityChange>,
    #[cfg(feature = "tracing")]
    resource_span: Span,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            last: SnapshotId(0),
            delta: vec![],
            #[cfg(feature = "tracing")]
            resource_span: span!(Level::DEBUG, "WorldState"),
        }
    }

    pub fn get(&self, id: SnapshotId) -> Option<WorldViewRef<'_>> {
        self.snapshots
            .get(&id)
            .map(|snapshot| WorldViewRef { snapshot })
    }

    pub fn get_mut(&mut self, id: SnapshotId) -> Option<WorldViewMut<'_>> {
        self.snapshots.get_mut(&id).map(|snapshot| WorldViewMut {
            snapshot,
            delta: &mut self.delta,
        })
    }

    pub fn insert(&mut self, id: SnapshotId) {
        let entities = self.snapshots.get(&self.last).cloned().unwrap_or(Snapshot {
            entities: Entities { entities: vec![] },
        });
        self.snapshots.insert(id, entities);
        self.last = id;
        self.delta.clear();
    }

    pub fn remove(&mut self, id: SnapshotId) {
        dbg!("rm");
        self.snapshots.remove(&id);
    }

    // FIXME: This should run while modifying WorldViewMut (e.g. on Drop).
    pub fn patch_delta(&mut self, mut id: SnapshotId) {
        // Requested snapshot already up to date.
        id += 1;

        // Change all up to last (including).
        while id <= self.last {
            let snap = self.snapshots.get_mut(&id).unwrap();

            for delta in self.delta.clone() {
                snap.apply(delta);
            }

            tracing::info!("Applying delta to {:?}", id);

            id += 1;
        }
    }

    pub fn delta(&self) -> &[EntityChange] {
        &self.delta
    }
}

#[derive(Debug)]
pub struct WorldViewRef<'a> {
    snapshot: &'a Snapshot,
}

impl<'a> WorldViewRef<'a> {
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot.entities.entities.iter().find(|x| x.id == id)
    }
}

#[derive(Debug)]
pub struct WorldViewMut<'a> {
    // entities: &'a mut Entities,
    // delta: Vec<EntityChange>,
    snapshot: &'a mut Snapshot,
    delta: &'a mut Vec<EntityChange>,
}

impl<'a> WorldViewMut<'a> {
    pub fn get_mut(&mut self, id: EntityId) -> Option<EntityMut<'_>> {
        self.snapshot
            .entities
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
        self.snapshot.entities.entities.push(entity.clone());
        self.delta.push(EntityChange::Create {
            id: entity.id,
            data: entity,
        });
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.snapshot.entities.entities.retain(|i| i.id != id);
        self.delta.push(EntityChange::Destroy { id });
    }

    pub fn delta(&self) -> &[EntityChange] {
        &self.delta
    }
}

impl<'a> Drop for WorldViewMut<'a> {
    fn drop(&mut self) {}
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

#[derive(Clone, Debug)]
struct Entities {
    entities: Vec<Entity>,
}

impl Entities {
    fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities
            .iter_mut()
            .find(|x| x.id == id)
            .map(|entity| entity)
    }

    fn despawn(&mut self, id: EntityId) {
        self.entities.retain(|i| i.id != id);
    }
}

#[derive(Clone, Debug)]
struct Snapshot {
    entities: Entities,
}

impl Snapshot {
    fn apply(&mut self, delta: EntityChange) {
        match delta {
            EntityChange::Create { id, data } => {
                self.entities.entities.push(Entity {
                    id,
                    transform: data.transform,
                    data: data.data,
                });
            }
            EntityChange::Destroy { id } => {
                // TODO
                todo!()
            }
            EntityChange::Translate { id, translation } => {
                let entity = self.entities.get_mut(id).unwrap();
                entity.transform.translation = translation;
            }
            EntityChange::Rotate { id, rotation } => {
                todo!()
            }
        }
    }
}
