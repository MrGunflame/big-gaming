pub mod metrics;

use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut};

use bevy_ecs::system::Resource;

use game_common::world::CellId;

#[cfg(feature = "tracing")]
use tracing::{event, span, Level, Span};

use crate::components::inventory::Inventory;
use crate::components::items::Item;
use crate::entity::EntityId;
use crate::world::snapshot::{EntityChange, TransferCell};

pub use metrics::WorldMetrics;

use super::control_frame::ControlFrame;
use super::entity::{Entity, EntityBody};
use super::inventory::InventoriesMut;
use super::source::{StreamingSource, StreamingState};

/// The world state at constant time intervals.
#[derive(Clone, Debug, Resource)]
pub struct WorldState {
    // TODO: This can be a fixed size ring buffer.
    pub(crate) snapshots: VecDeque<Snapshot>,
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

    pub fn get(&self, cf: ControlFrame) -> Option<WorldViewRef<'_>> {
        let mut index = 0;

        while index < self.snapshots.len() {
            let snapshot = &self.snapshots[index];

            if cf == snapshot.control_frame {
                return Some(WorldViewRef { snapshot, index });
            }

            index += 1;
        }

        None
    }

    // pub fn get(&self, cf: ControlFrame) -> Option<WorldViewRef<'_>> {
    //     let index = self.get_index(cf)?;
    //     self.snapshots
    //         .get(index)
    //         .map(|s| WorldViewRef { snapshot: s, index })
    // }

    pub fn get_mut(&mut self, cf: ControlFrame) -> Option<WorldViewMut<'_>> {
        let index = self.get_index(cf)?;
        self.snapshots.get_mut(index)?;

        Some(WorldViewMut {
            world: self,
            index,
            new_deltas: HashMap::new(),
        })
    }

    pub fn index(&self, cf: ControlFrame) -> Option<usize> {
        self.get_index(cf)
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, cf: ControlFrame) {
        #[cfg(debug_assertions)]
        if let Some(snapshot) = self.snapshots.back() {
            assert!(snapshot.control_frame < cf);
        }

        self.metrics.snapshots.inc();

        let snapshot = match self.snapshots.back() {
            Some(snapshot) => {
                let mut snap = snapshot.clone();
                snap.control_frame = cf;
                snap.cells.clear();
                snap
            }
            None => Snapshot {
                control_frame: cf,
                entities: Entities::default(),
                hosts: Hosts::new(),
                cells: HashMap::new(),
                streaming_sources: StreamingSources::new(),
                inventories: Inventories::new(),
            },
        };

        self.snapshots.push_back(snapshot);
    }

    pub fn remove(&mut self, cf: ControlFrame) {
        let Some(index) = self.get_index(cf) else {
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

    /// Returns the newest snapshot.
    pub fn back(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.back().map(|s| WorldViewRef {
            snapshot: s,
            index: self.len() - 1,
        })
    }

    /// Returns the newest snapshot.
    pub fn back_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.back_mut()?;

        Some(WorldViewMut {
            index: self.len() - 1,
            world: self,
            new_deltas: HashMap::new(),
        })
    }

    /// Returns the oldest snapshot.
    pub fn front(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.front().map(|s| WorldViewRef {
            snapshot: s,
            index: 0,
        })
    }

    /// Returns the oldest snapshot.
    pub fn front_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.front_mut()?;

        Some(WorldViewMut {
            index: 0,
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

    fn get_index(&self, cf: ControlFrame) -> Option<usize> {
        let mut index = 0;

        while index < self.snapshots.len() {
            let snapshot = &self.snapshots[index];

            if cf <= snapshot.control_frame {
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

    pub fn streaming_sources(&self) -> &StreamingSources {
        &self.snapshot.streaming_sources
    }

    pub fn inventories(&self) -> &Inventories {
        &self.snapshot.inventories
    }

    /// Returns a view into a cell in the world.
    pub fn cell(&self, id: CellId) -> CellViewRef<'_> {
        CellViewRef {
            id,
            entities: &self.snapshot.entities,
            cells: &self.snapshot.cells,
        }
    }

    pub fn deltas(&self) -> impl Iterator<Item = &EntityChange> {
        self.snapshot.cells.values().flatten()
    }

    pub fn control_frame(&self) -> ControlFrame {
        self.snapshot.control_frame
    }
}

pub struct WorldViewMut<'a> {
    pub(crate) world: &'a mut WorldState,
    pub(crate) index: usize,
    /// A list of changes applied while this `WorldViewMut` was held.
    ///
    /// Note that we can't use the snapshot-global delta list as that would be applied to every
    /// snapshot, even if it was already applied.
    pub(crate) new_deltas: HashMap<CellId, Vec<EntityChange>>,
}

impl<'a> WorldViewMut<'a> {
    pub(crate) fn snapshot_ref(&self) -> &Snapshot {
        self.world.snapshots.get(self.index).unwrap()
    }

    pub(crate) fn snapshot(&mut self) -> &mut Snapshot {
        self.world.snapshots.get_mut(self.index).unwrap()
    }

    pub fn deltas(&mut self) -> &mut HashMap<CellId, Vec<EntityChange>> {
        &mut self.snapshot().cells
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot_ref().entities.get(id)
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

    pub fn spawn(&mut self, mut entity: Entity) -> EntityId {
        self.world.metrics.entities.inc();
        self.world.metrics.deltas.inc();

        #[cfg(feature = "tracing")]
        event!(
            Level::TRACE,
            "[{}] spawning {:?} (C = {})",
            self.index,
            entity.id,
            entity.cell()
        );

        let id = self.snapshot().entities.spawn(entity.clone());
        entity.id = id;

        self.new_deltas
            .entry(entity.cell())
            .or_default()
            .push(EntityChange::Create { entity });

        id
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

        let entity = self.get(id).unwrap();
        self.new_deltas
            .entry(CellId::from(entity.transform.translation))
            .or_default()
            .push(EntityChange::CreateHost { id });
    }

    pub fn despawn_host(&mut self, id: EntityId) {
        self.snapshot().hosts.remove(id);

        if let Some(entity) = self.get(id) {
            self.new_deltas
                .entry(CellId::from(entity.transform.translation))
                .or_default()
                .push(EntityChange::CreateHost { id });
        }
    }

    /// Sets the streaming state of the entity.
    pub fn upate_streaming_source(&mut self, id: EntityId, state: StreamingState) {
        #[cfg(debug_assertions)]
        if state != StreamingState::Create {
            assert!(self.snapshot().streaming_sources.get(id).is_some());
        }

        let translation = self
            .snapshot()
            .entities
            .get(id)
            .unwrap()
            .transform
            .translation;

        self.new_deltas
            .entry(CellId::from(translation))
            .or_default()
            .push(EntityChange::UpdateStreamingSource { id, state });

        match state {
            StreamingState::Create => {
                self.snapshot().streaming_sources.insert(id, state);
            }
            StreamingState::Active => {
                self.snapshot().streaming_sources.insert(id, state);
            }
            StreamingState::Destroy => {
                self.snapshot().streaming_sources.insert(id, state);
            }
            StreamingState::Destroyed => {
                self.snapshot().streaming_sources.remove(id);
            }
        }
    }

    pub fn control_frame(&self) -> ControlFrame {
        self.snapshot_ref().control_frame
    }

    pub fn inventories(&self) -> &Inventories {
        &self.snapshot_ref().inventories
    }

    pub fn inventories_mut(&mut self) -> InventoriesMut<'_, 'a> {
        InventoriesMut { view: self }
    }
}

impl<'a> Debug for WorldViewMut<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldViewMut")
            .field("index", &self.index)
            .field("snapshot", self.snapshot_ref())
            .field("new_deltas", &self.new_deltas)
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
            let entity_id = self.id;

            // Update the cell when moved.
            let prev = CellId::from(self.prev.transform.translation);
            let curr = CellId::from(self.entity.transform.translation);

            // TODO: Weird things will happen if a prev != curr is being overwritten.
            if prev == curr {
                let cell = self.cells.entry(prev).or_default();

                let mut should_insert = true;
                for elem in cell.iter_mut() {
                    match elem {
                        EntityChange::Translate {
                            id,
                            translation,
                            cell,
                        } if *id == entity_id => {
                            *translation = self.entity.transform.translation;
                            should_insert = false;
                            break;
                        }
                        _ => (),
                    }
                }

                if should_insert {
                    cell.push(EntityChange::Translate {
                        id: entity_id,
                        translation: self.entity.transform.translation,
                        cell: TransferCell::new(prev, curr),
                    });
                }
            } else {
            }

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
            let entity_id = self.id;

            let cell = self
                .cells
                .entry(CellId::from(self.entity.transform.translation))
                .or_default();

            // Updated existsing event if it exists.
            let mut should_insert = true;
            for elem in cell.iter_mut() {
                match elem {
                    EntityChange::Rotate { id, rotation } if *id == entity_id => {
                        *rotation = self.entity.transform.rotation;
                        should_insert = false;
                        break;
                    }
                    _ => (),
                }
            }

            if should_insert {
                cell.push(EntityChange::Rotate {
                    id: entity_id,
                    rotation: self.entity.transform.rotation,
                });
            }
        }

        // TODO: Other deltas
    }
}

#[derive(Clone, Debug, Default)]
struct Entities {
    next_id: u64,
    entities: HashMap<EntityId, Entity>,
}

impl Entities {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    fn spawn(&mut self, mut entity: Entity) -> EntityId {
        let id = self.next_id();
        entity.id = id;

        self.entities.insert(entity.id, entity);
        id
    }

    fn despawn(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    fn next_id(&mut self) -> EntityId {
        let id = EntityId::from_raw(self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Snapshot {
    control_frame: ControlFrame,
    entities: Entities,
    hosts: Hosts,
    streaming_sources: StreamingSources,
    // Deltas for every cell
    pub cells: HashMap<CellId, Vec<EntityChange>>,
    pub(crate) inventories: Inventories,
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

/// Entities that keep chunks loaded.
#[derive(Clone, Debug)]
pub struct StreamingSources {
    entities: HashMap<EntityId, StreamingSource>,
}

impl StreamingSources {
    fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&StreamingSource> {
        self.entities.get(&id)
    }

    fn insert(&mut self, id: EntityId, state: StreamingState) {
        self.entities.insert(id, StreamingSource { state });
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
            EntityChange::Create { entity } => {
                self.entities.insert(entity);
            }
            EntityChange::Destroy { id } => {
                let Some(translation) = self.entities.get(id).map(|s| s.transform.translation) else {
                    tracing::warn!("no such entiy to despawn: {:?}", id);
                    return;
                };

                self.entities.despawn(id);
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
            }
            EntityChange::Rotate { id, rotation } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.transform.rotation = rotation;
                } else {
                    tracing::warn!("tried to rotate a non-existant entity");
                }
            }
            EntityChange::Health { id, health } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    if let EntityBody::Actor(actor) = &mut entity.body {
                        actor.health = health;
                    }
                }
            }
            EntityChange::CreateHost { id } => {
                self.hosts.insert(id);
            }
            EntityChange::DestroyHost { id } => {
                self.hosts.remove(id);
            }
            EntityChange::UpdateStreamingSource { id, state } => {
                let entity = self.entities.get(id).unwrap();

                match state {
                    StreamingState::Create => {
                        self.streaming_sources.insert(id, state);
                    }
                    StreamingState::Active => {
                        self.streaming_sources.insert(id, state);
                    }
                    StreamingState::Destroy => {
                        self.streaming_sources.insert(id, state);
                    }
                    StreamingState::Destroyed => {
                        self.streaming_sources.remove(id);
                    }
                };
            }
            EntityChange::InventoryItemAdd(event) => {
                let inventory = self.inventories.get_mut_or_insert(event.entity);

                let item = Item {
                    id: event.item,
                    resistances: None,
                    actions: Default::default(),
                    components: Default::default(),
                    mass: Default::default(),
                    equipped: false,
                    hidden: false,
                };

                // FIXME: Don't panic
                inventory.insert(item).unwrap();
            }
            EntityChange::InventoryItemRemove(event) => {
                if let Some(inventory) = self.inventories.get_mut(event.entity) {
                    inventory.remove(event.id);
                }
            }
            EntityChange::InventoryDestroy(event) => {
                self.inventories.remove(event.entity);
            }
        }
    }
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
            .filter(|(_, e)| e.cell() == self.id)
            .map(|(_, e)| e)
    }

    pub fn deltas(&self) -> &[EntityChange] {
        self.cells
            .get(&self.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[derive(Clone, Debug, Default)]
pub struct Inventories {
    inventories: HashMap<EntityId, Inventory>,
}

impl Inventories {
    fn new() -> Self {
        Self {
            inventories: HashMap::new(),
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Inventory> {
        self.inventories.get_mut(&id)
    }

    pub fn get_mut_or_insert(&mut self, id: EntityId) -> &mut Inventory {
        if !self.inventories.contains_key(&id) {
            self.inventories.insert(id, Inventory::new());
        }

        self.get_mut(id).unwrap()
    }

    pub fn insert(&mut self, id: EntityId, inventory: Inventory) {
        self.inventories.insert(id, inventory);
    }

    pub fn remove(&mut self, id: EntityId) {
        self.inventories.remove(&id);
    }
}

pub trait AsView {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, id: EntityId) -> Option<&Entity>;

    fn cell(&self, id: CellId) -> CellViewRef<'_>;
}

impl<'a> AsView for WorldViewRef<'a> {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot.entities.get(id)
    }

    fn len(&self) -> usize {
        self.snapshot.entities.entities.len()
    }

    fn cell(&self, id: CellId) -> CellViewRef<'_> {
        CellViewRef {
            id,
            entities: &self.snapshot.entities,
            cells: &self.snapshot.cells,
        }
    }
}

impl<'a> AsView for &'a WorldViewMut<'a> {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot_ref().entities.get(id)
    }

    fn len(&self) -> usize {
        self.snapshot_ref().entities.entities.len()
    }

    fn cell(&self, id: CellId) -> CellViewRef<'_> {
        CellViewRef {
            id,
            entities: &self.snapshot_ref().entities,
            cells: &self.snapshot_ref().cells,
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use crate::components::components::Components;
    use crate::components::object::ObjectId;
    use crate::components::transform::Transform;
    use crate::record::RecordReference;
    use crate::world::entity::Object;

    use super::*;

    macro_rules! assert_get {
        ($world:expr, $in:expr) => {
            assert!({
                if let Some(v) = $world.get($in) {
                    v.control_frame() == $in
                } else {
                    false
                }
            })
        };
        ($world:expr, $in:expr, $out:expr) => {
            assert!({
                if let Some(v) = $world.get($in) {
                    v.control_frame() == $out
                } else {
                    false
                }
            })
        };
    }

    #[test]
    fn test_world_frames() {
        let mut world = WorldState::new();

        assert_eq!(world.len(), 0);
        assert_eq!(world.is_empty(), true);

        let cf0 = ControlFrame(0);
        let cf1 = ControlFrame(5);
        let cf2 = ControlFrame(10);

        world.insert(cf0);
        assert_eq!(world.len(), 1);
        assert_get!(world, cf0);

        world.insert(cf1);
        assert_eq!(world.len(), 2);
        assert_get!(world, cf1);

        world.insert(cf2);
        assert_eq!(world.len(), 3);
        assert_get!(world, cf2);
    }

    #[test]
    fn test_world() {
        let mut world = WorldState::new();

        let cf0 = ControlFrame(0);
        let cf1 = ControlFrame(1);

        world.insert(cf0);

        let mut view = world.at_mut(0).unwrap();
        assert_eq!(view.control_frame(), cf0);

        let id = view.spawn(Entity {
            id: EntityId::dangling(),
            transform: Transform::default(),
            body: EntityBody::Object(Object {
                id: ObjectId(RecordReference::STUB),
            }),
            components: Components::new(),
        });

        assert!(view.get(id).is_some());
        drop(view);

        // Spawned entity should exist in new snapshot.
        world.insert(cf1);

        let view = world.at(0).unwrap();
        assert!(view.get(id).is_some());

        drop(view);
    }

    #[test]
    fn test_world_cells() {
        let mut world = WorldState::new();

        let cf0 = ControlFrame(0);
        let cf1 = ControlFrame(5);

        world.insert(cf0);
        world.insert(cf1);

        let mut view = world.get_mut(cf0).unwrap();

        let id = view.spawn(Entity {
            id: EntityId::dangling(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            body: EntityBody::Object(Object {
                id: ObjectId(RecordReference::STUB),
            }),
            components: Components::new(),
        });

        drop(view);

        // Translate entity from cell (0, 0, 0) to cell (1, 0, 0)
        let mut view = world.get_mut(cf1).unwrap();
        let mut entity = view.get_mut(id).unwrap();
        entity.transform.translation = Vec3::new(64.0, 0.0, 0.0);
        drop(entity);
        drop(view);

        // Entity unmoved in cf0
        {
            let view = world.get(cf0).unwrap();
            let entity = view.get(id).unwrap();
            assert_eq!(entity.transform.translation, Vec3::new(0.0, 0.0, 0.0));

            let cell = view.cell(CellId::from(Vec3::new(0.0, 0.0, 0.0)));
            assert!(cell.get(id).is_some());

            let cell = view.cell(CellId::from(Vec3::new(64.0, 0.0, 0.0)));
            assert!(cell.get(id).is_none());
        }

        // Moved in cf1
        {
            let view = world.get(cf1).unwrap();
            let entity = view.get(id).unwrap();
            assert_eq!(entity.transform.translation, Vec3::new(64.0, 0.0, 0.0));

            let cell = view.cell(CellId::from(Vec3::new(0.0, 0.0, 0.0)));
            assert!(cell.get(id).is_none());

            let cell = view.cell(CellId::from(Vec3::new(64.0, 0.0, 0.0)));
            assert!(cell.get(id).is_some());
        }
    }

    #[test]
    fn world_view_delta_events() {
        let mut world = WorldState::new();

        let cf0 = ControlFrame(0);
        let cf1 = ControlFrame(1);

        world.insert(cf0);
        world.insert(cf1);

        let mut view0 = world.get_mut(cf0).unwrap();
        view0.spawn(create_test_entity());
        drop(view0);

        // Events propagate to newest snapshot
        let view0 = world.get(cf0).unwrap();

        let events = view0.deltas().collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        for event in events {
            match event {
                EntityChange::Create { entity: _ } => (),
                _ => panic!("unexpected event: {:?}", event),
            }
        }
    }

    fn create_test_entity() -> Entity {
        Entity {
            id: EntityId::dangling(),
            transform: Transform::default(),
            body: EntityBody::Object(Object {
                id: ObjectId(RecordReference::STUB),
            }),
            components: Components::new(),
        }
    }

    fn create_test_world(num_snapshots: u32) -> (WorldState, Vec<ControlFrame>) {
        let mut world = WorldState::new();

        let cfs = (0..num_snapshots)
            .map(|index| {
                let cf = ControlFrame(index);
                world.insert(cf);
                cf
            })
            .collect();

        (world, cfs)
    }

    #[test]
    fn world_patch_forward() {
        let (mut world, cfs) = create_test_world(10);

        for cf in &cfs {
            let view = world.get(*cf).unwrap();
            assert_eq!(view.len(), 0);
        }

        let mut view = world.get_mut(cfs[cfs.len() / 2]).unwrap();
        view.spawn(create_test_entity());
        drop(view);

        for cf in &cfs[..cfs.len() / 2] {
            let view = world.get(*cf).unwrap();
            assert_eq!(view.len(), 0);
            assert_eq!(view.deltas().count(), 0);
        }

        {
            let view = world.get(cfs[cfs.len() / 2]).unwrap();
            assert_eq!(view.len(), 1);
            assert_eq!(view.deltas().count(), 1);
        }

        for cf in &cfs[(cfs.len() / 2) + 1..] {
            let view = world.get(*cf).unwrap();
            assert_eq!(view.len(), 1);
            assert_eq!(view.deltas().count(), 0);
        }
    }

    #[test]
    fn world_patch_not_backwards() {
        let (mut world, cfs) = create_test_world(10);

        for cf in &cfs {
            let view = world.get(*cf).unwrap();
            assert_eq!(view.len(), 0);
        }

        let mut view = world.get_mut(cfs[cfs.len() - 1]).unwrap();
        view.spawn(create_test_entity());
        drop(view);

        for cf in &cfs[..cfs.len() - 2] {
            let view = world.get(*cf).unwrap();
            assert_eq!(view.len(), 0);
            assert_eq!(view.deltas().count(), 0);
        }
    }
}
