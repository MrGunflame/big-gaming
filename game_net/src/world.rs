use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityData, EntityId};

use game_common::world::CellId;
use glam::{Quat, Vec3};

#[cfg(feature = "tracing")]
use tracing::{event, span, Level, Span};

use crate::metrics::WorldMetrics;
use crate::snapshot::{EntityChange, TransferCell};

/// The world state at constant time intervals.
#[derive(Clone, Debug, Resource)]
pub struct WorldState {
    // TODO: This can be a fixed size ring buffer.
    snapshots: VecDeque<Snapshot>,
    head: usize,
    metrics: WorldMetrics,

    #[cfg(feature = "tracing")]
    resource_span: Span,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::new(),
            head: 0,
            #[cfg(feature = "tracing")]
            resource_span: span!(Level::DEBUG, "WorldState"),
            metrics: WorldMetrics::new(),
        }
    }

    pub fn get(&self, ts: Instant) -> Option<WorldViewRef<'_>> {
        let index = self.get_index(ts)?;
        self.snapshots
            .get(index)
            .map(|s| WorldViewRef { snapshot: s, index })
    }

    pub fn get_mut(&mut self, ts: Instant) -> Option<WorldViewMut<'_>> {
        let index = self.get_index(ts)?;
        self.snapshots.get_mut(index)?;

        Some(WorldViewMut {
            world: self,
            index,
            new_deltas: HashMap::new(),
        })
    }

    pub fn index(&self, ts: Instant) -> Option<usize> {
        self.get_index(ts)
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, ts: Instant) {
        #[cfg(debug_assertions)]
        if let Some(snapshot) = self.snapshots.back() {
            assert!(snapshot.creation < ts);
        }

        self.metrics.snapshots.inc();

        let snapshot = match self.snapshots.back() {
            Some(snapshot) => {
                let mut snap = snapshot.clone();
                snap.creation = ts;
                snap.cells.clear();
                snap
            }
            None => Snapshot {
                creation: ts,
                entities: Entities::default(),
                hosts: Hosts::new(),
                cells: HashMap::new(),
            },
        };

        self.snapshots.push_back(snapshot);
    }

    pub fn remove(&mut self, ts: Instant) {
        let Some(index) = self.get_index(ts) else {
            return;
        };

        let snapshot = self.snapshots.remove(index).unwrap();
        self.drop_snapshot(snapshot);

        if self.head > 0 {
            self.head -= 1;
        }
    }

    /// Removes the oldest snapshot.
    pub fn pop(&mut self) {
        if let Some(snapshot) = self.snapshots.pop_front() {
            self.drop_snapshot(snapshot);

            if self.head > 0 {
                self.head -= 1;
            }
        }
    }

    pub fn back(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.front().map(|s| WorldViewRef {
            snapshot: s,
            index: 0,
        })
    }

    pub fn back_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.front_mut()?;

        Some(WorldViewMut {
            world: self,
            index: 0,
            new_deltas: HashMap::new(),
        })
    }

    pub fn front(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.back().map(|s| WorldViewRef {
            snapshot: s,
            index: self.len() - 1,
        })
    }

    pub fn front_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.back_mut()?;

        Some(WorldViewMut {
            index: self.len() - 1,
            world: self,
            new_deltas: HashMap::new(),
        })
    }

    pub fn at(&self, index: usize) -> Option<WorldViewRef<'_>> {
        self.snapshots
            .get(index)
            .map(|s| WorldViewRef { snapshot: s, index })
    }

    pub fn at_mut(&mut self, index: usize) -> Option<WorldViewMut<'_>> {
        self.snapshots.get_mut(index)?;

        Some(WorldViewMut {
            world: self,
            index,
            new_deltas: HashMap::new(),
        })
    }

    fn get_index(&self, ts: Instant) -> Option<usize> {
        let mut index = 0;

        while index < self.snapshots.len() {
            let snapshot = &self.snapshots[index];

            if ts <= snapshot.creation {
                return Some(index);
            }

            index += 1;
        }

        None
    }

    pub fn metrics(&self) -> &WorldMetrics {
        &self.metrics
    }

    fn drop_snapshot(&self, snapshot: Snapshot) {
        self.metrics.snapshots.dec();

        let deltas = snapshot.cells.values().map(|e| e.len() as u64).sum();
        self.metrics.deltas.sub(deltas);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WorldViewRef<'a> {
    snapshot: &'a Snapshot,
    index: usize,
}

impl<'a> WorldViewRef<'a> {
    pub fn len(&self) -> usize {
        self.snapshot.entities.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot.entities.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.snapshot.entities.entities.values()
    }

    /// Returns a view into a cell in the world.
    pub fn cell(&self, id: CellId) -> CellViewRef<'_> {
        CellViewRef {
            id,
            entities: &self.snapshot.entities,
            cells: &self.snapshot.cells,
        }
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
                                cell: TransferCell::new(
                                    entity.transform.translation,
                                    new.transform.translation,
                                ),
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

    #[inline]
    pub fn creation(&self) -> Instant {
        self.snapshot.creation
    }
}

pub struct WorldViewMut<'a> {
    world: &'a mut WorldState,
    index: usize,
    /// A list of changes applied while this `WorldViewMut` was held.
    ///
    /// Note that we can't use the snapshot-global delta list as that would be applied to every
    /// snapshot, even if it was already applied.
    new_deltas: HashMap<CellId, Vec<EntityChange>>,
}

impl<'a> WorldViewMut<'a> {
    fn snapshot_ref(&self) -> &Snapshot {
        self.world.snapshots.get(self.index).unwrap()
    }

    fn snapshot(&mut self) -> &mut Snapshot {
        self.world.snapshots.get_mut(self.index).unwrap()
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<EntityMut<'_>> {
        let sn = self.world.snapshots.get_mut(self.index).unwrap();

        match sn.entities.get_mut(id) {
            Some(entity) => Some(EntityMut {
                cells: &mut self.new_deltas,
                prev: entity.clone(),
                entity,
            }),
            None => None,
        }
    }

    pub fn spawn(&mut self, entity: Entity) {
        self.world.metrics.entities.inc();
        self.world.metrics.deltas.inc();

        #[cfg(feature = "tracing")]
        event!(
            Level::TRACE,
            "[{}] spawning {:?} (C = {})",
            self.index,
            entity.id,
            CellId::from(entity.transform.translation).to_f32()
        );

        self.new_deltas
            .entry(CellId::from(entity.transform.translation))
            .or_default()
            .push(EntityChange::Create {
                id: entity.id,
                data: entity.clone(),
            });

        self.snapshot().entities.spawn(entity.clone());
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.world.metrics.entities.dec();
        self.world.metrics.deltas.inc();

        let translation = self
            .snapshot()
            .entities
            .get(id)
            .unwrap()
            .transform
            .translation;

        #[cfg(feature = "tracing")]
        event!(
            Level::TRACE,
            "[{}] despawning {:?} (C = {})",
            self.index,
            id,
            CellId::from(translation).to_f32()
        );

        self.snapshot().entities.despawn(id);

        self.new_deltas
            .entry(CellId::from(translation))
            .or_default()
            .push(EntityChange::Destroy { id });

        // Despawn host with the entity if exists.
        self.despawn_host(id);
    }

    pub fn spawn_host(&mut self, id: EntityId) {
        #[cfg(debug_assertions)]
        assert!(self.snapshot().entities.get(id).is_some());

        self.snapshot().hosts.insert(id);
    }

    pub fn despawn_host(&mut self, id: EntityId) {
        self.snapshot().hosts.remove(id);
    }

    #[inline]
    pub fn creation(&self) -> Instant {
        self.world.snapshots.get(self.index).unwrap().creation
    }
}

impl<'a> Debug for WorldViewMut<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldViewMut")
            .field("index", &self.index)
            .field("snapshot", self.snapshot_ref())
            .finish_non_exhaustive()
    }
}

impl<'a> Drop for WorldViewMut<'a> {
    fn drop(&mut self) {
        // Deltas from the current snapshot are only in `new_deltas`.
        // Copy all `new_deltas` into cells.
        let view = self.world.snapshots.get_mut(self.index).unwrap();

        for (k, v) in &self.new_deltas {
            self.world.metrics.deltas.add(v.len() as u64);

            view.cells.entry(*k).or_default().extend(v.clone());
        }

        let mut index = self.index + 1;

        while index < self.world.snapshots.len() {
            #[cfg(feature = "tracing")]
            event!(
                Level::TRACE,
                "[{}] patch {} into {}",
                self.index,
                self.index,
                index
            );

            let view = self.world.snapshots.get_mut(index).unwrap();

            // Copy deltas
            for (_, v) in self.new_deltas.iter() {
                self.world.metrics.deltas.add(v.len() as u64);

                for change in v {
                    #[cfg(feature = "tracing")]
                    event!(
                        Level::TRACE,
                        "[{}] apply {}",
                        self.index,
                        event_to_str(change)
                    );

                    view.apply(change.clone());
                }
            }

            #[cfg(feature = "tracing")]
            event!(
                Level::TRACE,
                "[{}] done patching {} into {}",
                self.index,
                self.index,
                index
            );

            index += 1;
        }
    }
}

#[cfg(feature = "tracing")]
fn event_to_str(event: &EntityChange) -> &'static str {
    match event {
        EntityChange::Create { id: _, data: _ } => "Create",
        EntityChange::Destroy { id: _ } => "Destroy",
        EntityChange::Translate {
            id: _,
            translation: _,
            cell: _,
        } => "Translate",
        EntityChange::Rotate { id: _, rotation: _ } => "Rotate",
        EntityChange::Health { id: _, health: _ } => "Health",
        EntityChange::CreateHost { id: _ } => "CreateHost",
        EntityChange::DestroyHost { id: _ } => "DestroyHost",
        EntityChange::CreateTerrain { cell: _, height: _ } => "CreateTerrain",
    }
}

pub struct EntityMut<'a> {
    cells: &'a mut HashMap<CellId, Vec<EntityChange>>,
    prev: Entity,
    entity: &'a mut Entity,
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
            // Update the cell when moved.
            let prev = CellId::from(self.prev.transform.translation);
            let curr = CellId::from(self.entity.transform.translation);

            self.cells
                .entry(prev)
                .or_default()
                .push(EntityChange::Translate {
                    id: self.entity.id,
                    translation: self.entity.transform.translation,
                    cell: TransferCell::new(prev, curr),
                });

            self.cells
                .entry(curr)
                .or_default()
                .push(EntityChange::Translate {
                    id: self.entity.id,
                    translation: self.entity.transform.translation,
                    cell: TransferCell::new(prev, curr),
                });

            if prev != curr {
                // TODO
            }
        }

        if self.prev.transform.rotation != self.entity.transform.rotation {
            self.cells
                .entry(CellId::from(self.entity.transform.translation))
                .or_default()
                .push(EntityChange::Rotate {
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
    creation: Instant,
    entities: Entities,
    hosts: Hosts,
    // Deltas for every cell
    cells: HashMap<CellId, Vec<EntityChange>>,
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
        // Note that an entity may have already been despawned in the next snapshot.
        // In that case we simply ignore the change.

        match delta {
            EntityChange::Create { id, data } => {
                self.cells
                    .entry(CellId::from(data.transform.translation))
                    .or_default()
                    .push(EntityChange::Create {
                        id,
                        data: data.clone(),
                    });

                self.entities.spawn(Entity {
                    id,
                    transform: data.transform,
                    data: data.data,
                });
            }
            EntityChange::Destroy { id } => {
                let Some(translation) = self.entities.get(id).map(|s| s.transform.translation) else {
                    tracing::warn!("no such entiy to despawn: {:?}", id);
                    return;
                };

                self.entities.despawn(id);

                self.cells
                    .entry(CellId::from(translation))
                    .or_default()
                    .push(EntityChange::Destroy { id });
            }
            EntityChange::Translate {
                id,
                translation,
                cell,
            } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.transform.translation = translation;
                } else {
                    tracing::warn!("tried to translate a non-existant entity");
                }

                self.cells
                    .entry(CellId::from(translation))
                    .or_default()
                    .push(EntityChange::Translate {
                        id,
                        translation,
                        cell,
                    });
            }
            EntityChange::Rotate { id, rotation } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.transform.rotation = rotation;

                    self.cells
                        .entry(CellId::from(entity.transform.translation))
                        .or_default()
                        .push(EntityChange::Rotate { id, rotation });
                } else {
                    tracing::warn!("tried to rotate a non-existant entity");
                }
            }
            EntityChange::Health { id, health } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    if let EntityData::Actor { race: _, health: h } = &mut entity.data {
                        *h = health;
                    }
                }
            }
            EntityChange::CreateHost { id } => {
                self.hosts.insert(id);
            }
            EntityChange::DestroyHost { id } => {
                self.hosts.remove(id);
            }
            EntityChange::CreateTerrain { cell, height } => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct Override {
    pub id: EntityId,
    pub translation: Option<Vec3>,
    pub rotation: Option<Quat>,
}

#[derive(Clone, Debug)]
pub struct CellViewRef<'a> {
    id: CellId,
    entities: &'a Entities,
    cells: &'a HashMap<CellId, Vec<EntityChange>>,
}

impl<'a> CellViewRef<'a> {
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn id(&self) -> CellId {
        self.id
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        let entity = self.entities.get(id)?;
        if CellId::from(entity.transform.translation) != self.id {
            None
        } else {
            Some(entity)
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities
            .entities
            .iter()
            .filter(|(_, e)| CellId::from(e.transform.translation) == self.id)
            .map(|(_, e)| e)
    }

    pub fn deltas(&self) -> &[EntityChange] {
        self.cells
            .get(&self.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn delta(this: Self, next: CellViewRef<'_>) -> Vec<EntityChange> {
        let mut entities = HashMap::<EntityId, Entity>::from_iter(
            next.iter()
                .cloned()
                .filter(|e| CellId::from(e.transform.translation) == this.id)
                .map(|e| (e.id, e)),
        );

        let mut delta = Vec::new();

        for entity in this.iter() {
            match entities.remove(&entity.id) {
                Some(new) => {
                    if entity.transform.translation != new.transform.translation {
                        delta.push(EntityChange::Translate {
                            id: entity.id,
                            translation: new.transform.translation,
                            cell: TransferCell::new(
                                entity.transform.translation,
                                new.transform.translation,
                            ),
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

        for entity in entities.into_values() {
            delta.push(EntityChange::Create {
                id: entity.id,
                data: entity,
            });
        }

        delta
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    macro_rules! assert_get {
        ($world:expr, $in:expr) => {
            assert!({
                if let Some(v) = $world.get($in) {
                    v.creation() == $in
                } else {
                    false
                }
            })
        };
        ($world:expr, $in:expr, $out:expr) => {
            assert!({
                if let Some(v) = $world.get($in) {
                    v.creation() == $out
                } else {
                    false
                }
            })
        };
    }

    #[test]
    fn test_world_times() {
        let mut world = WorldState::new();

        assert_eq!(world.len(), 0);
        assert_eq!(world.is_empty(), true);

        let now = Instant::now();

        let t1 = now;
        let t2 = now + Duration::from_millis(10);
        let t3 = now + Duration::from_millis(20);

        world.insert(t1);
        assert_eq!(world.len(), 1);
        assert_get!(world, t1);

        world.insert(t2);
        assert_eq!(world.len(), 2);
        assert_get!(world, t2);

        world.insert(t3);
        assert_eq!(world.len(), 3);
        assert_get!(world, t3);

        assert_get!(world, t1 + Duration::from_millis(5), t2);
    }
}
