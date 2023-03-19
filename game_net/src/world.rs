use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityData, EntityId};

use game_common::world::CellId;
use glam::{Quat, Vec3};
#[cfg(feature = "tracing")]
use tracing::{span, Level, Span};

use crate::proto::sequence::Sequence;
use crate::snapshot::EntityChange;

/// The world state at constant time intervals.
#[derive(Clone, Debug, Resource)]
pub struct WorldState {
    // TODO: This can be a fixed size ring buffer.
    snapshots: VecDeque<Snapshot>,
    delta: Vec<EntityChange>,
    overries: Overrides,
    head: usize,
    #[cfg(feature = "tracing")]
    resource_span: Span,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::new(),
            delta: vec![],
            overries: Overrides::new(),
            head: 0,
            #[cfg(feature = "tracing")]
            resource_span: span!(Level::DEBUG, "WorldState"),
        }
    }

    pub fn get(&self, ts: Instant) -> Option<WorldViewRef<'_>> {
        let index = self.get_index(ts)?;
        self.snapshots
            .get(index)
            .map(|s| WorldViewRef { snapshot: s })
    }

    pub fn get_mut(&mut self, ts: Instant) -> Option<WorldViewMut<'_>> {
        let index = self.get_index(ts)?;
        self.snapshots.get_mut(index)?;

        Some(WorldViewMut { world: self, index })
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

        let snapshot = match self.snapshots.back() {
            Some(snapshot) => {
                let mut snap = snapshot.clone();
                snap.creation = ts;
                snap
            }
            None => Snapshot {
                creation: ts,
                entities: Entities::default(),
                hosts: Hosts::new(),
                cells: HashSet::new(),
            },
        };

        self.delta = vec![];
        self.snapshots.push_back(snapshot);
    }

    pub fn remove(&mut self, ts: Instant) {
        self.snapshots.retain(|s| s.creation != ts);
        self.head -= 1;
    }

    /// Removes the oldest snapshot.
    pub fn pop(&mut self) {
        if self.snapshots.pop_front().is_some() && self.head > 0 {
            self.head -= 1;
        }
    }

    // FIXME: This should run while modifying WorldViewMut (e.g. on Drop).
    #[deprecated]
    pub fn patch_delta(&mut self, ts: Instant) {
        // Now handled by WorldViewMut::drop.

        // let Some(mut index) = self.get_index(ts) else {
        //     return;
        // };

        // // Change all up to last (including).
        // while index < self.snapshots.len() {
        //     let snap = self.snapshots.get_mut(index).unwrap();

        //     for delta in self.delta.clone() {
        //         snap.apply(delta);
        //     }

        //     index += 1;
        // }
    }

    pub fn delta(&self) -> &[EntityChange] {
        &self.delta
    }

    pub fn next(&mut self) -> Option<NextWorldView<'_>> {
        let next = self.snapshots.get(self.head)?;
        let prev = self.snapshots.get(self.head.wrapping_sub(1));

        self.head += 1;

        Some(NextWorldView {
            prev: prev.map(|s| WorldViewRef { snapshot: s }),
            view: WorldViewRef { snapshot: next },
            delta: prev.map(|p| next.creation - p.creation),
        })
    }

    pub fn back(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.front().map(|s| WorldViewRef { snapshot: s })
    }

    pub fn back_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.front_mut()?;

        Some(WorldViewMut {
            world: self,
            index: 0,
        })
    }

    pub fn front(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.back().map(|s| WorldViewRef { snapshot: s })
    }

    pub fn at(&self, index: usize) -> Option<WorldViewRef<'_>> {
        self.snapshots
            .get(index)
            .map(|s| WorldViewRef { snapshot: s })
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
}

#[derive(Copy, Clone, Debug)]
pub struct NextWorldView<'a> {
    pub prev: Option<WorldViewRef<'a>>,
    pub view: WorldViewRef<'a>,
    /// The delta time elapsed since the last snapshot. `None` if no previous snapshot exists.
    ///
    /// For the client this is the interpolation period between the previous and the current
    /// snapshot.
    pub delta: Option<Duration>,
}

#[derive(Copy, Clone, Debug)]
pub struct WorldViewRef<'a> {
    snapshot: &'a Snapshot,
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

#[derive(Debug)]
pub struct WorldViewMut<'a> {
    world: &'a mut WorldState,
    index: usize,
    // entities: &'a mut Entities,
    // delta: Vec<EntityChange>,
    // snapshot: &'a mut Snapshot,
    // delta: &'a mut Vec<EntityChange>,
}

impl<'a> WorldViewMut<'a> {
    fn snapshot(&mut self) -> &mut Snapshot {
        self.world.snapshots.get_mut(self.index).unwrap()
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<EntityMut<'_>> {
        let sn = self.world.snapshots.get_mut(self.index).unwrap();

        match sn.entities.get_mut(id) {
            Some(entity) => Some(EntityMut {
                cells: &mut sn.cells,
                prev: entity.clone(),
                entity,
                delta: &mut self.world.delta,
            }),
            None => None,
        }
    }

    pub fn spawn(&mut self, entity: Entity) {
        self.snapshot()
            .cells
            .insert(CellId::from(entity.transform.translation));

        self.snapshot().entities.spawn(entity.clone());

        self.world.delta.push(EntityChange::Create {
            id: entity.id,
            data: entity,
        });
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.snapshot().entities.despawn(id);
        self.world.delta.push(EntityChange::Destroy { id });

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

    pub fn delta(&self) -> &[EntityChange] {
        &self.world.delta
    }

    #[inline]
    pub fn creation(&self) -> Instant {
        self.world.snapshots.get(self.index).unwrap().creation
    }
}

impl<'a> Drop for WorldViewMut<'a> {
    fn drop(&mut self) {
        while self.index < self.world.snapshots.len() {
            let view = self.world.snapshots.get_mut(self.index).unwrap();

            for delta in &self.world.delta {
                view.apply(delta.clone());
            }

            self.index += 1;
        }
    }
}

pub struct EntityMut<'a> {
    cells: &'a mut HashSet<CellId>,
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

            // Update the cell when moved.
            let prev = CellId::from(self.prev.transform.translation);
            let curr = CellId::from(self.entity.transform.translation);
            if prev != curr {
                self.cells.insert(curr);
            }
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
    creation: Instant,
    entities: Entities,
    hosts: Hosts,
    cells: HashSet<CellId>,
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
                self.cells.insert(CellId::from(data.transform.translation));

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
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.transform.translation = translation;
                }

                self.cells.insert(CellId::from(translation));
            }
            EntityChange::Rotate { id, rotation } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.transform.rotation = rotation;
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct Overrides {
    seqs: HashMap<Sequence, Vec<Override>>,
    ids: HashMap<EntityId, Override>,
}

impl Overrides {
    pub fn new() -> Self {
        Self {
            seqs: HashMap::new(),
            ids: HashMap::new(),
        }
    }

    pub fn insert(&mut self, seq: Sequence, e: Override) {
        match self.seqs.get_mut(&seq) {
            Some(vec) => {
                vec.push(e.clone());
            }
            None => {
                self.seqs.insert(seq, vec![e.clone()]);
            }
        }

        self.ids.insert(e.id, e);
    }

    pub fn remove(&mut self, seq: Sequence) {
        let (_, e) = self.seqs.remove_entry(&seq).unwrap_or_default();

        for e in e {
            self.ids.remove(&e.id);
        }
    }

    pub fn get(&self, id: EntityId) -> Option<Override> {
        self.ids.get(&id).cloned()
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

    pub fn delta(this: Option<Self>, next: CellViewRef<'_>) -> Vec<EntityChange> {
        let mut entities =
            HashMap::<EntityId, Entity>::from_iter(next.iter().cloned().map(|e| (e.id, e)));

        let mut delta = Vec::new();

        if let Some(view) = this {
            for entity in view.iter() {
                match entities.remove(&entity.id) {
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
        assert!(matches!(world.next(), None));

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
