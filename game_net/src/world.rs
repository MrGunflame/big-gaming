use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityData, EntityId};

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

    pub fn newest(&self) -> Option<WorldViewRef<'_>> {
        self.get(self.last)
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
            entities: Entities::default(),
            hosts: Hosts::default(),
        });
        self.snapshots.insert(id, entities);
        self.last = id;
        self.delta.clear();
    }

    pub fn remove(&mut self, id: SnapshotId) {
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

#[derive(Copy, Clone, Debug)]
pub struct WorldViewRef<'a> {
    snapshot: &'a Snapshot,
}

impl<'a> WorldViewRef<'a> {
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot.entities.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.snapshot.entities.entities.values()
    }

    /// Creates a delta from `self` to `next`.
    pub fn delta(this: Option<Self>, next: WorldViewRef<'_>) -> Vec<EntityChange> {
        let mut entities = next.snapshot.entities.clone();
        let mut hosts = next.snapshot.hosts.clone();

        let mut delta = Vec::new();

        if let Some(view) = this {
            for entity in view.iter() {
                match entities.entities.remove(&entity.id) {
                    Some(new) => {
                        if entity.transform.translation != new.transform.translation {
                            delta.push(EntityChange::Translate {
                                id: entity.id,
                                translation: new.transform.translation,
                            });
                        }

                        if entity.transform.rotation != new.transform.rotation {
                            delta.push(EntityChange::Rotate {
                                id: entity.id,
                                rotation: new.transform.rotation,
                            });
                        }

                        match (&entity.data, &new.data) {
                            (
                                EntityData::Actor { race: _, health },
                                EntityData::Actor {
                                    race: _,
                                    health: new,
                                },
                            ) => {
                                if health != new {
                                    delta.push(EntityChange::Health {
                                        id: entity.id,
                                        health: *new,
                                    });
                                }
                            }
                            _ => (),
                        }
                    }
                    None => {
                        delta.push(EntityChange::Destroy { id: entity.id });
                    }
                }
            }

            for id in view.snapshot.hosts.entities.keys().copied() {
                match hosts.entities.remove(&id) {
                    Some(()) => {}
                    None => {
                        delta.push(EntityChange::Destroy { id });
                    }
                }
            }
        }

        for entity in entities.entities.into_values() {
            delta.push(EntityChange::Create {
                id: entity.id,
                data: entity,
            });
        }

        for id in hosts.entities.into_keys() {
            delta.push(EntityChange::CreateHost { id });
        }

        delta
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
        self.snapshot.entities.get_mut(id).map(|entity| EntityMut {
            prev: entity.clone(),
            entity,
            delta: &mut self.delta,
        })
    }

    pub fn spawn(&mut self, entity: Entity) {
        self.snapshot.entities.spawn(entity.clone());
        self.delta.push(EntityChange::Create {
            id: entity.id,
            data: entity,
        });
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.snapshot.entities.despawn(id);
        self.delta.push(EntityChange::Destroy { id });

        // Despawn host with the entity if exists.
        self.despawn_host(id);
    }

    pub fn spawn_host(&mut self, id: EntityId) {
        #[cfg(debug_assertions)]
        assert!(self.snapshot.entities.get(id).is_some());

        self.snapshot.hosts.insert(id);
    }

    pub fn despawn_host(&mut self, id: EntityId) {
        self.snapshot.hosts.remove(id);
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

        if self.prev.transform.rotation != self.entity.transform.rotation {
            self.delta.push(EntityChange::Rotate {
                id: self.entity.id,
                rotation: self.entity.transform.rotation,
            });
        }

        // TODO: Other deltas
    }
}

#[derive(Clone, Debug, Default)]
struct Entities {
    entities: HashMap<EntityId, Entity>,
}

impl Entities {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    fn spawn(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    fn despawn(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }
}

#[derive(Clone, Debug)]
struct Snapshot {
    entities: Entities,
    hosts: Hosts,
}

#[derive(Clone, Debug, Default)]
struct Hosts {
    // TODO: Add HostId (or similar)
    entities: HashMap<EntityId, ()>,
}

impl Hosts {
    fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    fn get(&self, id: EntityId) -> Option<&()> {
        self.entities.get(&id)
    }

    fn insert(&mut self, id: EntityId) {
        self.entities.insert(id, ());
    }

    fn remove(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }
}

impl Snapshot {
    fn apply(&mut self, delta: EntityChange) {
        match delta {
            EntityChange::Create { id, data } => {
                self.entities.spawn(Entity {
                    id,
                    transform: data.transform,
                    data: data.data,
                });
            }
            EntityChange::Destroy { id } => {
                self.entities.despawn(id);
            }
            EntityChange::Translate { id, translation } => {
                let entity = self.entities.get_mut(id).unwrap();
                entity.transform.translation = translation;
            }
            EntityChange::Rotate { id, rotation } => {
                let entity = self.entities.get_mut(id).unwrap();
                entity.transform.rotation = rotation;
            }
            EntityChange::Health { id, health } => {
                let entity = self.entities.get_mut(id).unwrap();

                if let EntityData::Actor { race: _, health: h } = &mut entity.data {
                    *h = health;
                }
            }
            EntityChange::CreateHost { id } => {
                self.hosts.insert(id);
            }
            EntityChange::DestroyHost { id } => {
                self.hosts.remove(id);
            }
        }
    }
}
