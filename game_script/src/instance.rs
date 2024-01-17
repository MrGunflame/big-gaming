use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use game_common::components::components::RawComponent;
use game_common::components::inventory::{Inventory, InventorySlotId};
use game_common::components::items::ItemStack;
use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::record::RecordReference;
use game_common::world::World;
use game_tracing::trace_span;
use game_wasm::player::PlayerId;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::builtin::register_host_fns;
use crate::dependency::{Dependencies, Dependency};
use crate::effect::{Effect, Effects, PlayerSetActive};
use crate::events::{
    DispatchEvent, OnAction, OnCollision, OnEquip, OnInit, OnUnequip, OnUpdate, WasmFnTrampoline,
};
use crate::{Entry, Handle, Pointer, RecordProvider, System, WorldProvider};

pub(crate) struct InstancePool {
    /// Linker for instantiating new instances.
    linker: Linker<State>,
    instances: HashMap<Handle, Runnable>,
}

impl InstancePool {
    pub(crate) fn new(engine: &Engine) -> Self {
        let mut linker = Linker::<State>::new(engine);
        register_host_fns(&mut linker);

        Self {
            instances: HashMap::new(),
            linker,
        }
    }

    pub fn init(
        &mut self,
        engine: &Engine,
        module: &Module,
        handle: Handle,
    ) -> wasmtime::Result<InitState> {
        let state = State::Init(InitState {
            script: handle,
            systems: vec![],
            actions: HashMap::new(),
            event_handlers: HashMap::new(),
        });

        let mut store = Store::new(engine, state);
        let instance = self.linker.instantiate(&mut store, module).unwrap();
        let mut runnable = Runnable { store, instance };

        runnable.init()?;
        let state = match runnable.store.data_mut() {
            State::Init(state) => state.clone(),
            State::Run(_) => unreachable!(),
        };

        debug_assert!(!self.instances.contains_key(&handle));
        self.instances.insert(handle, runnable);

        Ok(state)
    }

    pub fn get<'a>(&'a mut self, state: State, handle: Handle) -> &mut Runnable {
        let runnable = self.instances.get_mut(&handle).unwrap();
        *runnable.store.data_mut() = state;
        runnable
    }
}

impl Debug for InstancePool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstancePool").finish_non_exhaustive()
    }
}

pub struct Runnable {
    store: Store<State>,
    instance: Instance,
}

impl Runnable {
    pub(crate) fn run(&mut self, event: &Event) -> wasmtime::Result<()> {
        let _span = trace_span!("Instance::run").entered();

        match event {
            Event::Action(event) => self.on_action(event.entity, event.invoker),
            Event::Collision(event) => self.on_collision(event.entity, event.other),
            Event::Equip(event) => self.on_equip(event.item, event.entity),
            Event::Unequip(event) => self.on_unequip(event.item, event.entity),
            Event::Update(entity) => self.on_update(*entity),
        }
    }

    pub(crate) fn init(&mut self) -> wasmtime::Result<()> {
        let _span = trace_span!("Runnable::init").entered();

        let func: OnInit = self.instance.get_typed_func(&mut self.store, "on_init")?;
        func.call(&mut self.store, ())
    }

    /// Calls the guest function with the given pointer.
    pub(crate) fn call(&mut self, ptr: Pointer, entity: EntityId) -> wasmtime::Result<()> {
        let _span = trace_span!("Runnable::call").entered();

        let func: WasmFnTrampoline = self
            .instance
            .get_typed_func(&mut self.store, "__wasm_fn_trampoline")?;
        func.call(&mut self.store, (ptr.0, entity.into_raw()))
    }

    fn on_update(&mut self, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnUpdate = self.instance.get_typed_func(&mut self.store, "on_update")?;
        func.call(&mut self.store, entity.into_raw())
    }

    fn on_action(&mut self, entity: EntityId, invoker: EntityId) -> wasmtime::Result<()> {
        let func: OnAction = self.instance.get_typed_func(&mut self.store, "on_action")?;
        func.call(&mut self.store, invoker.into_raw())
    }

    fn on_collision(&mut self, entity: EntityId, other: EntityId) -> wasmtime::Result<()> {
        let func: OnCollision = self
            .instance
            .get_typed_func(&mut self.store, "on_collision")?;
        func.call(&mut self.store, (entity.into_raw(), other.into_raw()))
    }

    fn on_equip(&mut self, item: InventorySlotId, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnEquip = self.instance.get_typed_func(&mut self.store, "on_equip")?;
        func.call(&mut self.store, (item.into_raw(), entity.into_raw()))
    }

    fn on_unequip(&mut self, item: InventorySlotId, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnUnequip = self
            .instance
            .get_typed_func(&mut self.store, "on_unequip")?;
        func.call(&mut self.store, (item.into_raw(), entity.into_raw()))
    }

    pub fn into_state(&mut self) -> RunState {
        let state = core::mem::replace(
            self.store.data_mut(),
            State::Init(InitState {
                script: Handle(0),
                systems: vec![],
                actions: HashMap::new(),
                event_handlers: HashMap::new(),
            }),
        );

        match state {
            State::Init(_) => unreachable!(),
            State::Run(state) => state,
        }
    }
}

pub enum State {
    Init(InitState),
    Run(RunState),
}

impl State {
    pub fn as_init(&mut self) -> wasmtime::Result<&mut InitState> {
        match self {
            Self::Init(state) => Ok(state),
            Self::Run(_) => Err(wasmtime::Error::msg("not in init state")),
        }
    }

    pub fn as_run(&self) -> wasmtime::Result<&RunState> {
        match self {
            Self::Init(_) => Err(wasmtime::Error::msg("not in run state")),
            Self::Run(s) => Ok(s),
        }
    }

    pub fn as_run_mut(&mut self) -> wasmtime::Result<&mut RunState> {
        match self {
            Self::Init(_) => Err(wasmtime::Error::msg("not in run state")),
            Self::Run(s) => Ok(s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InitState {
    pub script: Handle,
    pub systems: Vec<System>,
    pub actions: HashMap<RecordReference, Vec<Entry>>,
    pub event_handlers: HashMap<RecordReference, Vec<Entry>>,
}

pub struct RunState {
    world: *const dyn WorldProvider,
    pub records: *const dyn RecordProvider,
    pub physics_pipeline: *const game_physics::Pipeline,
    effects: *mut Effects,
    dependencies: *mut Dependencies,
    next_entity_id: u64,
    next_inventory_id: u64,
    pub new_world: World,
    pub action_buffer: Option<Vec<u8>>,
    pub events: Vec<DispatchEvent>,
}

impl RunState {
    pub fn new(
        world: *const dyn WorldProvider,
        physics_pipeline: *const game_physics::Pipeline,
        effects: *mut Effects,
        dependencies: *mut Dependencies,
        records: *const dyn RecordProvider,
        new_world: World,
        action_buffer: Option<Vec<u8>>,
    ) -> Self {
        Self {
            world,
            physics_pipeline,
            effects,
            next_entity_id: 0,
            next_inventory_id: 0,
            dependencies,
            records,
            new_world,
            action_buffer,
            events: Vec::new(),
        }
    }
}

impl RunState {
    pub fn spawn(&mut self) -> EntityId {
        let id = self.allocate_temporary_entity_id();
        self.new_world.spawn_with_id(id);

        self.effects().push(Effect::EntitySpawn(id));
        id
    }

    fn effects(&mut self) -> &mut Effects {
        unsafe { &mut *self.effects }
    }

    fn dependencies(&mut self) -> &mut Dependencies {
        unsafe { &mut *self.dependencies }
    }

    pub fn physics_pipeline(&self) -> &game_physics::Pipeline {
        unsafe { &*self.physics_pipeline }
    }

    pub fn records(&self) -> &dyn RecordProvider {
        unsafe { &*self.records }
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        if !self.new_world.contains(id) {
            return false;
        }

        self.effects().push(Effect::EntityDespawn(id));
        self.new_world.despawn(id);
        true
    }

    pub fn get_component(
        &mut self,
        entity_id: EntityId,
        component: RecordReference,
    ) -> Option<&RawComponent> {
        self.dependencies()
            .push(Dependency::EntityComponent(entity_id, component));
        self.new_world.get(entity_id, component)
    }

    pub fn insert_component(
        &mut self,
        entity_id: EntityId,
        id: RecordReference,
        component: RawComponent,
    ) {
        if !self.new_world.contains(entity_id) {
            return;
        }

        self.effects().push(Effect::EntityComponentInsert(
            entity_id,
            id,
            component.as_bytes().to_vec(),
        ));
        self.new_world.insert(entity_id, id, component);
    }

    pub fn remove_component(&mut self, entity_id: EntityId, id: RecordReference) -> bool {
        if !self.new_world.contains(entity_id) {
            return false;
        }

        if self.new_world.remove(entity_id, id).is_some() {
            self.effects()
                .push(Effect::EntityComponentRemove(entity_id, id));
            true
        } else {
            false
        }
    }

    fn reconstruct_inventory(&self, id: EntityId) -> Option<Inventory> {
        let mut inventory = unsafe { &*self.world }.inventory(id).cloned();

        for effect in unsafe { &*self.effects }.iter() {
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
        self.dependencies().push(Dependency::Inventory(entity));
        self.reconstruct_inventory(entity)
    }

    pub fn inventory_get(&mut self, entity: EntityId, slot: InventorySlotId) -> Option<ItemStack> {
        // We track the slot even if it does not exist.
        self.dependencies()
            .push(Dependency::InventorySlot(entity, slot));

        self.reconstruct_inventory(entity)?.get(slot).cloned()
    }

    pub fn inventory_insert(&mut self, entity: EntityId, stack: ItemStack) -> InventorySlotId {
        let id = self.allocate_temporary_inventory_id();
        self.effects()
            .push(Effect::InventoryInsert(entity, id, stack));
        id
    }

    pub fn inventory_remove(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        quantity: u64,
    ) -> bool {
        let Some(inventory) = unsafe { &*self.world }.inventory(entity) else {
            return false;
        };

        if inventory.clone().remove(slot, quantity as u32).is_some() {
            self.effects()
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

        self.effects().push(Effect::InventoryClear(entity));
        true
    }

    pub fn inventory_component_get(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        component: RecordReference,
    ) -> Option<RawComponent> {
        self.dependencies()
            .push(Dependency::InventorySlotComponent(entity, slot, component));

        let inventory = self.reconstruct_inventory(entity)?;
        inventory.get(slot)?.item.components.get(component).cloned()
    }

    pub fn inventory_component_insert(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        component_id: RecordReference,
        component: RawComponent,
    ) -> bool {
        let Some(inventory) = self.reconstruct_inventory(entity) else {
            return false;
        };

        if inventory.get(slot).is_none() {
            return false;
        };

        self.effects().push(Effect::InventoryComponentInsert(
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
        let Some(inventory) = unsafe { &*self.world }.inventory(entity) else {
            return false;
        };

        if inventory.get(slot).is_none() {
            return false;
        };

        self.effects()
            .push(Effect::InventoryComponentRemove(entity, slot, component_id));

        true
    }

    pub fn inventory_set_equipped(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        equipped: bool,
    ) -> bool {
        let Some(inventory) = unsafe { &*self.world }.inventory(entity) else {
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
            self.effects()
                .push(Effect::InventoryItemUpdateEquip(entity, slot, equipped));
            true
        }
    }

    pub fn player_lookup(&mut self, entity_id: EntityId) -> Option<PlayerId> {
        unsafe { &*self.world }.player(entity_id)
    }

    pub fn player_set_active(&mut self, player: PlayerId, entity: EntityId) -> bool {
        if !self.new_world.contains(entity) {
            return false;
        }

        self.effects()
            .push(Effect::PlayerSetActive(PlayerSetActive { player, entity }));
        true
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
