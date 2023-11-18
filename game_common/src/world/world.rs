pub mod metrics;

use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;

use game_common::world::CellId;

use glam::{Quat, Vec3};
#[cfg(feature = "tracing")]
use tracing::{event, span, Level, Span};

use crate::components::components::Component;
use crate::components::inventory::{Inventory, InventorySlotId};
use crate::components::items::{Item, ItemStack};
use crate::entity::EntityId;
use crate::record::RecordReference;
use crate::world::snapshot::EntityChange;

pub use metrics::WorldMetrics;

use super::control_frame::ControlFrame;
use super::entity::Entity;
use super::snapshot::{InventoryItemAdd, InventoryItemRemove, InventoryItemUpdate};
use super::source::StreamingSource;

/// The world state at constant time intervals.
#[derive(Clone, Debug)]
pub struct WorldState {
    // TODO: This can be a fixed size ring buffer.
    pub(crate) snapshots: VecDeque<Snapshot>,
    head: usize,
    metrics: WorldMetrics,

    // The entity id must go global across all snapshots.
    // FIXME: Snapshots should only contain a sparse array of entity ids
    // with deltas. Then we can store the dense array for all snapshots
    // directly here.
    next_entity_id: u64,

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
            next_entity_id: 0,
        }
    }

    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        let mut w = Self {
            snapshots: VecDeque::new(),
            head: 0,
            #[cfg(feature = "tracing")]
            resource_span: span!(Level::DEBUG, "WorldState"),
            metrics: WorldMetrics::new(),
            next_entity_id: 0,
        };

        w.snapshots.push_back(snapshot);
        w
    }

    pub fn get(&self, cf: ControlFrame) -> Option<WorldViewRef<'_>> {
        let mut index = 0;

        while index < self.snapshots.len() {
            let snapshot = &self.snapshots[index];

            if cf == snapshot.control_frame {
                return Some(WorldViewRef { snapshot });
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
            new_deltas: vec![],
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
                snap.deltas.clear();
                snap
            }
            None => Snapshot {
                control_frame: cf,
                entities: Entities::default(),
                deltas: vec![],
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
        self.drop_snapshot(&snapshot);

        if self.head > 0 {
            self.head -= 1;
        }
    }

    /// Removes the oldest snapshot.
    pub fn pop(&mut self) -> Option<Snapshot> {
        if let Some(snapshot) = self.snapshots.pop_front() {
            self.drop_snapshot(&snapshot);

            if self.head > 0 {
                self.head -= 1;
            }

            Some(snapshot)
        } else {
            None
        }
    }

    /// Returns the newest snapshot.
    pub fn back(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.back().map(|s| WorldViewRef { snapshot: s })
    }

    /// Returns the newest snapshot.
    pub fn back_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.back_mut()?;

        Some(WorldViewMut {
            index: self.len() - 1,
            world: self,
            new_deltas: vec![],
        })
    }

    /// Returns the oldest snapshot.
    pub fn front(&self) -> Option<WorldViewRef<'_>> {
        self.snapshots.front().map(|s| WorldViewRef { snapshot: s })
    }

    /// Returns the oldest snapshot.
    pub fn front_mut(&mut self) -> Option<WorldViewMut<'_>> {
        self.snapshots.front_mut()?;

        Some(WorldViewMut {
            index: 0,
            world: self,
            new_deltas: vec![],
        })
    }

    pub fn at(&self, index: usize) -> Option<WorldViewRef<'_>> {
        self.snapshots
            .get(index)
            .map(|s| WorldViewRef { snapshot: s })
    }

    pub fn at_mut(&mut self, index: usize) -> Option<WorldViewMut<'_>> {
        self.snapshots.get_mut(index)?;

        Some(WorldViewMut {
            world: self,
            index,
            new_deltas: vec![],
        })
    }

    fn get_index(&self, cf: ControlFrame) -> Option<usize> {
        let mut index = 0;

        while index < self.snapshots.len() {
            let snapshot = &self.snapshots[index];

            if cf == snapshot.control_frame {
                return Some(index);
            }

            index += 1;
        }

        None
    }

    pub fn metrics(&self) -> &WorldMetrics {
        &self.metrics
    }

    fn drop_snapshot(&self, snapshot: &Snapshot) {
        self.metrics.snapshots.dec();

        let deltas = snapshot.deltas.len() as u64;
        self.metrics.deltas.sub(deltas);
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WorldViewRef<'a> {
    snapshot: &'a Snapshot,
}

impl<'a> WorldViewRef<'a> {
    pub fn snapshot(&self) -> &'a Snapshot {
        self.snapshot
    }

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
        }
    }

    pub fn deltas(&self) -> impl Iterator<Item = &EntityChange> {
        self.snapshot.deltas.iter()
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
    pub(crate) new_deltas: Vec<EntityChange>,
}

impl<'a> WorldViewMut<'a> {
    pub fn len(&self) -> usize {
        self.snapshot_ref().entities.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn snapshot_ref(&self) -> &Snapshot {
        self.world.snapshots.get(self.index).unwrap()
    }

    pub(crate) fn snapshot(&mut self) -> &mut Snapshot {
        self.world.snapshots.get_mut(self.index).unwrap()
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.snapshot_ref().entities.get(id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<EntityMut<'_>> {
        let sn = self.world.snapshots.get_mut(self.index).unwrap();

        match sn.entities.get_mut(id) {
            Some(entity) => Some(EntityMut {
                deltas: &mut self.new_deltas,
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

        let id = EntityId::from_raw(self.world.next_entity_id);
        self.world.next_entity_id += 1;
        // Don't overflow, we can't reuse ids at this point.
        assert_ne!(self.world.next_entity_id, u64::MAX);

        entity.id = id;
        self.snapshot().entities.spawn(entity.clone());

        self.new_deltas.push(EntityChange::Create { entity });

        id
    }

    /// Despawns and returns the entity.
    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        // streaming source needs to be destoryed first, otherwise
        // `remove_streaming_source` does nothing.
        if self.get(id).is_some() {
            self.remove_streaming_source(id);
        }

        let entity = self.snapshot().entities.despawn(id)?;

        self.world.metrics.entities.dec();
        self.world.metrics.deltas.inc();

        #[cfg(feature = "tracing")]
        {
            let translation = entity.transform.translation;

            event!(
                Level::TRACE,
                "[{}] despawning {:?} (C = {})",
                self.index,
                id,
                CellId::from(translation).to_f32()
            );
        }

        self.new_deltas.push(EntityChange::Destroy { id });

        // Despawn host with the entity if exists.
        self.despawn_host(id);

        Some(entity)
    }

    pub fn spawn_host(&mut self, id: EntityId) {
        #[cfg(debug_assertions)]
        assert!(self.snapshot().entities.get(id).is_some());

        let Some(entity) = self.snapshot().entities.get_mut(id) else {
            panic!("cannot create host for unknown entity: {:?}", id);
        };

        entity.is_host = true;
        self.new_deltas.push(EntityChange::CreateHost { id });
    }

    pub fn despawn_host(&mut self, id: EntityId) {
        if let Some(entity) = self.snapshot().entities.get_mut(id) {
            entity.is_host = false;
            self.new_deltas.push(EntityChange::DestroyHost { id });
        }
    }

    /// Sets the streaming state of the entity.
    pub fn insert_streaming_source(&mut self, id: EntityId, source: StreamingSource) {
        assert!(self.get(id).is_some());

        self.snapshot().streaming_sources.insert(id, source);

        assert!(self.get(id).is_some());
        self.new_deltas
            .push(EntityChange::CreateStreamingSource { id, source });
    }

    pub fn remove_streaming_source(&mut self, id: EntityId) -> Option<StreamingSource> {
        self.get(id)?;

        let source = self.snapshot().streaming_sources.remove(id)?;

        self.new_deltas
            .push(EntityChange::RemoveStreamingSource { id });

        Some(source)
    }

    pub fn control_frame(&self) -> ControlFrame {
        self.snapshot_ref().control_frame
    }

    pub fn inventories(&self) -> &Inventories {
        &self.snapshot_ref().inventories
    }

    pub fn inventories_mut(&mut self) -> &mut Inventories {
        &mut self.snapshot().inventories
    }

    // FIXME: Rework inventory API.
    pub fn inventory_insert_items(
        &mut self,
        id: EntityId,
        slot: InventorySlotId,
        items: ItemStack,
    ) {
        let item_id = items.item.id;
        let quantity = items.quantity;

        let inventory = self.inventories_mut().get_mut_or_insert(id);
        inventory.insert_at_slot(slot, items.clone()).unwrap();

        self.new_deltas
            .push(EntityChange::InventoryItemAdd(InventoryItemAdd {
                entity: id,
                id: slot,
                item: item_id,
                quantity,
                components: items.item.components,
                equipped: items.item.equipped,
                hidden: items.item.hidden,
            }));
    }

    pub fn inventory_insert_without_id(
        &mut self,
        id: EntityId,
        items: ItemStack,
    ) -> InventorySlotId {
        let item_id = items.item.id;
        let quantity = items.quantity;

        let inventory = self.inventories_mut().get_mut_or_insert(id);
        let slot = inventory.insert(items.clone()).unwrap();

        self.new_deltas
            .push(EntityChange::InventoryItemAdd(InventoryItemAdd {
                entity: id,
                id: slot,
                item: item_id,
                quantity,
                components: items.item.components,
                equipped: items.item.equipped,
                hidden: items.item.hidden,
            }));
        slot
    }

    pub fn inventory_remove_items(&mut self, id: EntityId, slot: InventorySlotId, quantity: u32) {
        let Some(inventory) = self.inventories_mut().get_mut(id) else {
            return;
        };

        inventory.remove(slot, quantity);

        self.new_deltas
            .push(EntityChange::InventoryItemRemove(InventoryItemRemove {
                entity: id,
                id: slot,
            }));
    }

    pub fn inventory_set_equipped(&mut self, id: EntityId, slot: InventorySlotId, equipped: bool) {
        let Some(inventory) = self.inventories_mut().get_mut(id) else {
            return;
        };

        let Some(stack) = inventory.get_mut(slot) else {
            return;
        };

        stack.item.equipped = equipped;
        let hidden = stack.item.hidden;

        self.new_deltas
            .push(EntityChange::InventoryItemUpdate(InventoryItemUpdate {
                entity: id,
                slot_id: slot,
                equipped,
                hidden,
                quantity: None,
                components: None,
            }));
    }

    pub fn inventory_component_insert(
        &mut self,
        id: EntityId,
        slot: InventorySlotId,
        component: RecordReference,
        data: Component,
    ) {
        let Some(inventory) = self.inventories_mut().get_mut(id) else {
            return;
        };

        let Some(stack) = inventory.get_mut(slot) else {
            return;
        };

        stack.item.components.insert(component, data);

        let equipped = stack.item.equipped;
        let hidden = stack.item.hidden;
        let components = stack.item.components.clone();

        self.new_deltas
            .push(EntityChange::InventoryItemUpdate(InventoryItemUpdate {
                entity: id,
                slot_id: slot,
                equipped,
                hidden,
                quantity: None,
                components: Some(components),
            }));
    }

    pub fn inventory_component_remove(
        &mut self,
        id: EntityId,
        slot: InventorySlotId,
        component: RecordReference,
    ) {
        let Some(inventory) = self.inventories_mut().get_mut(id) else {
            return;
        };

        let Some(stack) = inventory.get_mut(slot) else {
            return;
        };

        stack.item.components.remove(component);
        let equipped = stack.item.equipped;
        let hidden = stack.item.hidden;
        let components = stack.item.components.clone();

        self.new_deltas
            .push(EntityChange::InventoryItemUpdate(InventoryItemUpdate {
                entity: id,
                slot_id: slot,
                equipped,
                quantity: None,
                components: Some(components),
                hidden,
            }));
    }

    pub fn streaming_sources(&self) -> &StreamingSources {
        &self.snapshot_ref().streaming_sources
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
        view.deltas.extend(self.new_deltas.clone());

        self.world.metrics.deltas.add(self.new_deltas.len() as u64);

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
            for event in self.new_deltas.iter() {
                #[cfg(feature = "tracing")]
                event!(
                    Level::TRACE,
                    "[{}] apply {}",
                    self.index,
                    event_to_str(change)
                );

                view.apply(event.clone());
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
    deltas: &'a mut Vec<EntityChange>,
    entity: &'a mut Entity,
}

impl<'a> EntityMut<'a> {
    pub fn set_translation(&mut self, translation: Vec3) {
        if self.transform.translation == translation {
            return;
        }

        self.entity.transform.translation = translation;

        for event in self.deltas.iter_mut() {
            match event {
                EntityChange::Translate { id, translation } if *id == self.entity.id => {
                    *translation = self.entity.transform.translation;
                    return;
                }
                _ => (),
            }
        }

        self.deltas.push(EntityChange::Translate {
            id: self.entity.id,
            translation: self.transform.translation,
        });
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        if self.transform.rotation == rotation {
            return;
        }

        self.entity.transform.rotation = rotation;

        for event in self.deltas.iter_mut() {
            match event {
                EntityChange::Rotate { id, rotation } if *id == self.entity.id => {
                    *rotation = self.entity.transform.rotation;
                    return;
                }
                _ => (),
            }
        }

        self.deltas.push(EntityChange::Rotate {
            id: self.entity.id,
            rotation: self.transform.rotation,
        });
    }

    pub fn insert_component(&mut self, id: RecordReference, component: Component) {
        self.entity.components.insert(id, component.clone());

        self.deltas.push(EntityChange::ComponentAdd {
            entity: self.entity.id,
            component_id: id,
            component,
        });
    }

    pub fn remove_component(&mut self, id: RecordReference) {
        self.entity.components.remove(id);

        self.deltas.push(EntityChange::ComponentRemove {
            entity: self.entity.id,
            component_id: id,
        });
    }

    pub fn update_component(&mut self, id: RecordReference, component: Component) {
        self.entity.components.insert(id, component.clone());

        self.deltas.push(EntityChange::ComponentUpdate {
            entity: self.entity.id,
            component_id: id,
            component,
        });
    }
}

impl<'a> Deref for EntityMut<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        self.entity
    }
}

#[derive(Clone, Debug, Default)]
pub struct Entities {
    entities: HashMap<EntityId, Entity>,
}

impl Entities {
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    /// The id must be set before insertion.
    pub fn spawn(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        self.entities.remove(&id)
    }
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    control_frame: ControlFrame,
    pub entities: Entities,
    streaming_sources: StreamingSources,
    pub deltas: Vec<EntityChange>,
    pub inventories: Inventories,
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

    fn insert(&mut self, id: EntityId, source: StreamingSource) {
        self.entities.insert(id, source);
    }

    fn remove(&mut self, id: EntityId) -> Option<StreamingSource> {
        self.entities.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &StreamingSource)> {
        self.entities.iter().map(|(id, s)| (*id, s))
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
                if self.entities.get(id).is_none() {
                    tracing::warn!("no such entiy to despawn: {:?}", id);
                    return;
                };

                self.entities.despawn(id);
            }
            EntityChange::Translate { id, translation } => {
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
            EntityChange::CreateHost { id } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.is_host = true;
                }
            }
            EntityChange::DestroyHost { id } => {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.is_host = false;
                }
            }
            EntityChange::InventoryItemAdd(event) => {
                let inventory = self.inventories.get_mut_or_insert(event.entity);

                let item = Item {
                    id: event.item,
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
                    inventory.remove(event.id, 1);
                }
            }
            EntityChange::InventoryDestroy(event) => {
                self.inventories.remove(event.entity);
            }
            EntityChange::CreateStreamingSource { id, source } => {
                self.streaming_sources.insert(id, source);
            }
            EntityChange::RemoveStreamingSource { id } => {
                self.streaming_sources.remove(id);
            }
            EntityChange::ComponentAdd {
                entity,
                component_id,
                component,
            } => {
                if let Some(entity) = self.entities.get_mut(entity) {
                    entity.components.insert(component_id, component);
                } else {
                    tracing::warn!("no such entity: {:?}", entity);
                }
            }
            EntityChange::ComponentRemove {
                entity,
                component_id,
            } => {
                if let Some(entity) = self.entities.get_mut(entity) {
                    entity.components.remove(component_id);
                } else {
                    tracing::warn!("no such entity: {:?}", entity);
                }
            }
            EntityChange::ComponentUpdate {
                entity,
                component_id,
                component,
            } => {
                if let Some(entity) = self.entities.get_mut(entity) {
                    entity.components.insert(component_id, component);
                } else {
                    tracing::warn!("no such entity: {:?}", entity);
                }
            }
            EntityChange::InventoryItemUpdate(event) => {
                if let Some(inventory) = self.inventories.get_mut(event.entity) {
                    if let Some(stack) = inventory.get_mut(event.slot_id) {
                        stack.item.equipped = event.equipped;
                        stack.item.hidden = event.hidden;

                        if let Some(quantity) = event.quantity {
                            stack.quantity = quantity;
                        }
                    }
                } else {
                    tracing::warn!("no such entity: {:?}", event.entity);
                }
            }
        }
    }
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
            .filter(|(_, e)| e.cell() == self.id)
            .map(|(_, e)| e)
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
        self.inventories.entry(id).or_insert_with(Inventory::new);

        self.get_mut(id).unwrap()
    }

    pub fn insert(&mut self, id: EntityId, inventory: Inventory) {
        self.inventories.insert(id, inventory);
    }

    pub fn remove(&mut self, id: EntityId) {
        self.inventories.remove(&id);
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &Inventory)> {
        self.inventories.iter().map(|(k, v)| (*k, v))
    }
}

pub trait AsView {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, id: EntityId) -> Option<&Entity>;

    fn cell(&self, id: CellId) -> CellViewRef<'_>;

    fn iter(&self) -> EntitiesIter<'_>;

    fn inventory(&self, id: EntityId) -> Option<&Inventory>;
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
        }
    }

    fn iter(&self) -> EntitiesIter {
        EntitiesIter {
            inner: self.snapshot.entities.entities.values(),
        }
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
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
        }
    }

    fn iter(&self) -> EntitiesIter<'_> {
        EntitiesIter {
            inner: self.snapshot_ref().entities.entities.values(),
        }
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }
}

#[derive(Debug, Clone)]
pub struct EntitiesIter<'a> {
    inner: std::collections::hash_map::Values<'a, EntityId, Entity>,
}

impl<'a> Iterator for EntitiesIter<'a> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec3};

    use crate::components::components::Components;
    use crate::components::object::ObjectId;
    use crate::components::transform::Transform;
    use crate::record::RecordReference;
    use crate::world::entity::{EntityBody, Object};

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
            is_host: false,
        });

        assert!(view.get(id).is_some());
        drop(view);

        // Spawned entity should exist in new snapshot.
        world.insert(cf1);

        let view = world.at(0).unwrap();
        assert!(view.get(id).is_some());
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
            is_host: false,
        });

        drop(view);

        // Translate entity from cell (0, 0, 0) to cell (1, 0, 0)
        let mut view = world.get_mut(cf1).unwrap();
        let mut entity = view.get_mut(id).unwrap();
        entity.set_translation(Vec3::new(64.0, 0.0, 0.0));
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
            is_host: false,
        }
    }

    fn create_test_world(num_snapshots: u16) -> (WorldState, Vec<ControlFrame>) {
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

    #[test]
    fn world_unique_id_over_snapshots() {
        let mut world = WorldState::new();

        for index in 0..10 {
            let cf = ControlFrame(index);
            world.insert(cf);
        }

        let mut ids = Vec::new();
        for index in 0..10 {
            let cf = ControlFrame(index);

            let mut view = world.get_mut(cf).unwrap();
            ids.extend((0..10).map(|_| view.spawn(create_test_entity())));
        }

        // No duplicates
        for (index, id) in ids.iter().enumerate() {
            for (index2, id2) in ids.iter().enumerate() {
                if index == index2 {
                    continue;
                }

                assert_ne!(id, id2);
            }
        }
    }

    #[test]
    fn world_apply_rotation() {
        let mut world = WorldState::new();
        world.insert(ControlFrame(0));
        world.insert(ControlFrame(1));
        world.insert(ControlFrame(2));

        let mut view = world.get_mut(ControlFrame(0)).unwrap();
        let entity_id = view.spawn(create_test_entity());
        drop(view);

        let mut view = world.get_mut(ControlFrame(0)).unwrap();
        let mut entity = view.get_mut(entity_id).unwrap();
        entity.set_rotation(Quat::from_rotation_x(1.0));
        drop(entity);
        drop(view);

        let view = world.back().unwrap();
        let entity = view.get(entity_id).unwrap();
        assert_eq!(entity.transform.rotation, Quat::from_rotation_x(1.0));
    }

    #[test]
    fn world_delta_overwrite_previous() {
        let mut world = WorldState::new();
        world.insert(ControlFrame(0));
        world.insert(ControlFrame(1));

        let mut view = world.get_mut(ControlFrame(0)).unwrap();
        let entity_id = view.spawn(create_test_entity());

        for index in 0..16 {
            view.get_mut(entity_id)
                .unwrap()
                .set_translation(Vec3::splat(index as f32));
        }

        drop(view);

        let view = world.get(ControlFrame(0)).unwrap();
        assert_eq!(view.deltas().count(), 2);

        let mut creation_events = 0;
        let mut translation_events = 0;
        for event in view.deltas() {
            match event {
                EntityChange::Create { entity: _ } => creation_events += 1,
                EntityChange::Translate {
                    id: _,
                    translation: _,
                } => translation_events += 1,
                _ => panic!("invalid event: {:?}", event),
            }
        }

        assert_eq!(creation_events, 1);
        assert_eq!(translation_events, 1);
    }
}
