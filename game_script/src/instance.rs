use std::collections::HashMap;

use game_common::components::components::Component;
use game_common::components::inventory::{Inventory, InventorySlotId};
use game_common::components::items::ItemStack;
use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::record::RecordReference;
use game_common::world::entity::Entity;
use game_common::world::CellId;
use game_tracing::trace_span;
use glam::{Quat, Vec3};
use tracing::span::Id;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::dependency::{Dependencies, Dependency};
use crate::effect::{Effect, Effects};
use crate::events::{Events, OnAction, OnCellLoad, OnCellUnload, OnCollision, OnEquip, OnUnequip};
use crate::{RecordProvider, WorldProvider};

pub struct ScriptInstance<'a> {
    store: Store<State<'a>>,
    inner: Instance,
    events: Events,
}

impl<'a> ScriptInstance<'a> {
    pub fn new(
        engine: &Engine,
        module: &Module,
        events: Events,
        world: &'a dyn WorldProvider,
        physics_pipeline: &'a game_physics::Pipeline,
        effects: &'a mut Effects,
        dependencies: &'a mut Dependencies,
        records: &'a dyn RecordProvider,
    ) -> Self {
        let mut store = Store::new(
            engine,
            State::new(world, physics_pipeline, effects, dependencies, records),
        );

        let mut linker = Linker::<State>::new(&engine);

        crate::builtin::register_host_fns(&mut linker);

        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store,
            inner: instance,
            events,
        }
    }

    pub fn run(&mut self, event: &Event) -> wasmtime::Result<()> {
        let _span = trace_span!("Instance::run").entered();

        match event {
            Event::Action(event) => self.on_action(event.entity, event.invoker),
            Event::Collision(event) => self.on_collision(event.entity, event.other),
            Event::Equip(event) => self.on_equip(event.item, event.entity),
            Event::Unequip(event) => self.on_unequip(event.item, event.entity),
            Event::CellLoad(event) => self.on_cell_load(event.cell),
            Event::CellUnload(event) => self.on_cell_unload(event.cell),
        }
    }

    pub fn on_action(&mut self, entity: EntityId, invoker: EntityId) -> wasmtime::Result<()> {
        let func: OnAction = self.inner.get_typed_func(&mut self.store, "on_action")?;
        func.call(&mut self.store, invoker.into_raw())
    }

    pub fn on_collision(&mut self, entity: EntityId, other: EntityId) -> wasmtime::Result<()> {
        let func: OnCollision = self.inner.get_typed_func(&mut self.store, "on_collision")?;
        func.call(&mut self.store, (entity.into_raw(), other.into_raw()))
    }

    pub fn on_equip(&mut self, item: InventorySlotId, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnEquip = self.inner.get_typed_func(&mut self.store, "on_equip")?;
        func.call(&mut self.store, (item.into_raw(), entity.into_raw()))
    }

    pub fn on_unequip(&mut self, item: InventorySlotId, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnUnequip = self.inner.get_typed_func(&mut self.store, "on_unequip")?;
        func.call(&mut self.store, (item.into_raw(), entity.into_raw()))
    }

    pub fn on_cell_load(&mut self, id: CellId) -> wasmtime::Result<()> {
        let func: OnCellLoad = self.inner.get_typed_func(&mut self.store, "on_cell_load")?;
        func.call(&mut self.store, id.as_parts())
    }

    pub fn on_cell_unload(&mut self, id: CellId) -> wasmtime::Result<()> {
        let func: OnCellUnload = self
            .inner
            .get_typed_func(&mut self.store, "on_cell_unload")?;
        func.call(&mut self.store, id.as_parts())
    }
}

pub struct State<'a> {
    world: &'a dyn WorldProvider,
    pub records: &'a dyn RecordProvider,
    pub physics_pipeline: &'a game_physics::Pipeline,
    effects: &'a mut Effects,
    dependencies: &'a mut Dependencies,
    next_entity_id: u64,
    next_inventory_id: u64,
    /// Entities in its current state, if overwritten.
    ///
    /// `None` indicates that the entity was despawned.
    entities: HashMap<EntityId, Option<Entity>>,
}

impl<'a> State<'a> {
    pub fn new(
        world: &'a dyn WorldProvider,
        physics_pipeline: &'a game_physics::Pipeline,
        effects: &'a mut Effects,
        dependencies: &'a mut Dependencies,
        records: &'a dyn RecordProvider,
    ) -> Self {
        Self {
            world,
            physics_pipeline,
            effects,
            next_entity_id: 0,
            next_inventory_id: 0,
            entities: HashMap::with_capacity(16),
            dependencies,
            records,
        }
    }
}

impl<'a> State<'a> {
    pub fn spawn(&mut self, mut entity: Entity) -> EntityId {
        let id = self.allocate_temporary_entity_id();
        entity.id = id;

        self.entities.insert(id, Some(entity.clone()));
        self.effects.push(Effect::EntitySpawn(entity));
        id
    }

    pub fn get(&mut self, id: EntityId) -> Option<&Entity> {
        // We track the entity even if the entity does not exist.
        self.dependencies.push(Dependency::Entity(id));
        self.get_entity(id)
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        if self.get_entity(id).is_none() {
            return false;
        }

        *self.entities.entry(id).or_insert(None) = None;
        self.effects.push(Effect::EntityDespawn(id));
        true
    }

    pub fn set_translation(&mut self, id: EntityId, translation: Vec3) -> bool {
        let Some(entity) = self.get_entity(id).cloned() else {
            return false;
        };

        self.entities
            .entry(id)
            .or_insert_with(|| Some(entity))
            .as_mut()
            .unwrap()
            .transform
            .translation = translation;

        self.effects.push(Effect::EntityTranslate(id, translation));
        true
    }

    pub fn set_rotation(&mut self, id: EntityId, rotation: Quat) -> bool {
        let Some(entity) = self.get_entity(id).cloned() else {
            return false;
        };

        self.entities
            .entry(id)
            .or_insert_with(|| Some(entity))
            .as_mut()
            .unwrap()
            .transform
            .rotation = rotation;

        self.effects.push(Effect::EntityRotate(id, rotation));
        true
    }

    pub fn get_component(
        &mut self,
        entity_id: EntityId,
        component: RecordReference,
    ) -> Option<&Component> {
        self.dependencies
            .push(Dependency::EntityComponent(entity_id, component));

        self.get_entity(entity_id)
            .map(|entity| entity.components.get(component))
            .flatten()
    }

    pub fn insert_component(
        &mut self,
        entity_id: EntityId,
        id: RecordReference,
        component: Component,
    ) {
        let Some(entity) = self.get_entity(entity_id).cloned() else {
            return;
        };

        self.entities
            .entry(entity_id)
            .or_insert_with(|| Some(entity))
            .as_mut()
            .unwrap()
            .components
            .insert(id, component.clone());

        self.effects.push(Effect::EntityComponentInsert(
            entity_id,
            id,
            component.bytes,
        ));
    }

    pub fn remove_component(&mut self, entity_id: EntityId, id: RecordReference) -> bool {
        let Some(entity) = self.get_entity(entity_id).cloned() else {
            return false;
        };

        if self
            .entities
            .entry(entity_id)
            .or_insert_with(|| Some(entity))
            .as_mut()
            .unwrap()
            .components
            .remove(id)
            .is_some()
        {
            self.effects
                .push(Effect::EntityComponentRemove(entity_id, id));
            true
        } else {
            false
        }
    }

    fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        match self.entities.get(&id) {
            Some(Some(entity)) => Some(entity),
            Some(None) => None,
            None => self.world.get(id),
        }
    }

    fn reconstruct_inventory(&self, id: EntityId) -> Option<Inventory> {
        let mut inventory = self.world.inventory(id).cloned();

        for effect in self.effects.iter() {
            match effect {
                Effect::InventoryInsert(eid, slot_id, stack) if *eid == id => {
                    inventory.as_mut().unwrap().insert(stack.clone()).unwrap();
                }
                Effect::InventoryRemove(eid, slot_id, quantity) if *eid == id => {
                    inventory
                        .as_mut()
                        .unwrap()
                        .remove(slot_id, *quantity as u32);
                }
                Effect::InventoryClear(eid) if *eid == id => {
                    inventory.as_mut().unwrap().clear();
                }
                Effect::InventoryComponentInsert(eid, slot_id, comp_id, comp) if *eid == id => {
                    inventory
                        .as_mut()
                        .unwrap()
                        .get_mut(slot_id)
                        .unwrap()
                        .item
                        .components
                        .insert(*comp_id, comp.clone());
                }
                Effect::InventoryComponentRemove(eid, slot_id, comp_id) if *eid == id => {
                    inventory
                        .as_mut()
                        .unwrap()
                        .get_mut(slot_id)
                        .unwrap()
                        .item
                        .components
                        .remove(*comp_id);
                }
                _ => (),
            }
        }

        inventory
    }

    pub fn inventory(&mut self, entity: EntityId) -> Option<Inventory> {
        self.dependencies.push(Dependency::Inventory(entity));
        self.reconstruct_inventory(entity)
    }

    pub fn inventory_get(&mut self, entity: EntityId, slot: InventorySlotId) -> Option<ItemStack> {
        // We track the slot even if it does not exist.
        self.dependencies
            .push(Dependency::InventorySlot(entity, slot));

        self.reconstruct_inventory(entity)?.get(slot).cloned()
    }

    pub fn inventory_insert(&mut self, entity: EntityId, stack: ItemStack) -> InventorySlotId {
        let id = self.allocate_temporary_inventory_id();
        self.effects
            .push(Effect::InventoryInsert(entity, id, stack));
        id
    }

    pub fn inventory_remove(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        quantity: u64,
    ) -> bool {
        let Some(inventory) = self.world.inventory(entity) else {
            return false;
        };

        if inventory.clone().remove(slot, quantity as u32).is_some() {
            self.effects
                .push(Effect::InventoryRemove(entity, slot, quantity));
            true
        } else {
            false
        }
    }

    pub fn inventory_clear(&mut self, entity: EntityId) -> bool {
        if self.reconstruct_inventory(entity).is_none() {
            return false;
        };

        self.effects.push(Effect::InventoryClear(entity));
        true
    }

    pub fn inventory_component_get(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        component: RecordReference,
    ) -> Option<Component> {
        self.dependencies
            .push(Dependency::InventorySlotComponent(entity, slot, component));

        let inventory = self.reconstruct_inventory(entity)?;
        inventory.get(slot)?.item.components.get(component).cloned()
    }

    pub fn inventory_component_insert(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        component_id: RecordReference,
        component: Component,
    ) -> bool {
        let Some(inventory) = self.reconstruct_inventory(entity) else {
            return false;
        };

        if inventory.get(slot).is_none() {
            return false;
        };

        self.effects.push(Effect::InventoryComponentInsert(
            entity,
            slot,
            component_id,
            component,
        ));
        true
    }

    pub fn inventory_component_remove(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        component_id: RecordReference,
    ) -> bool {
        let Some(inventory) = self.world.inventory(entity) else {
            return false;
        };

        if inventory.get(slot).is_none() {
            return false;
        };

        self.effects
            .push(Effect::InventoryComponentRemove(entity, slot, component_id));

        true
    }

    pub fn inventory_set_equipped(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        equipped: bool,
    ) -> bool {
        let Some(inventory) = self.world.inventory(entity) else {
            return false;
        };

        let Some(stack) = inventory.get(slot) else {
            return false;
        };

        if stack.item.equipped == equipped {
            // Item is already in desired state, don't update
            // anything.
            false
        } else {
            self.effects
                .push(Effect::InventoryItemUpdateEquip(entity, slot, equipped));
            true
        }
    }

    /// Allocate a temporary [`EntityId`].
    // If a script spawns a new entity we need to acquire a temporary id, until
    // the game commits the effect. The id is only valid for this script local
    // script invocation and must be commited to a real id before the next script
    // invocation.
    fn allocate_temporary_entity_id(&mut self) -> EntityId {
        // FIXME: There is no guarantee how the bits for an entity id are used
        // currently. (In fact it is currently an ever-increasing counter). We should
        // reserve a section of the id to mark an id as temporary to avoid it colliding
        // with a real id.

        // For now we just use the top bit as a temporary sign.
        let bits = self.next_entity_id | (1 << 63);
        self.next_entity_id += 1;
        EntityId::from_raw(bits)
    }

    /// Allocate a temporary [`InventorySlotId`].
    fn allocate_temporary_inventory_id(&mut self) -> InventorySlotId {
        // FIXME: See comment in `allocate_temporary_entity_id`. The same applies
        // here.
        let bits = self.next_inventory_id | (1 << 63);
        self.next_inventory_id += 1;
        InventorySlotId::from_raw(bits)
    }
}
