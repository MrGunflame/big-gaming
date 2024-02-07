use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use game_common::components::components::RawComponent;
use game_common::components::inventory::{Inventory, InventorySlotId};
use game_common::components::items::ItemStack;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_tracing::trace_span;
use game_wasm::player::PlayerId;
use game_wasm::raw::{RESULT_NO_COMPONENT, RESULT_NO_ENTITY, RESULT_NO_INVENTORY_SLOT};
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::builtin::register_host_fns;
use crate::dependency::{Dependencies, Dependency};
use crate::effect::{
    Effect, Effects, EntityComponentInsert, EntityComponentRemove, PlayerSetActive,
};
use crate::events::{DispatchEvent, OnInit, WasmFnTrampoline};
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
    pub(crate) fn init(&mut self) -> wasmtime::Result<()> {
        let _span = trace_span!("Runnable::init").entered();

        let func: OnInit = self.instance.get_typed_func(&mut self.store, "on_init")?;
        func.call(&mut self.store, ())
    }

    /// Calls the guest function with the given pointer.
    pub(crate) fn call(&mut self, ptr: Pointer, entity: Option<EntityId>) -> wasmtime::Result<()> {
        let _span = trace_span!("Runnable::call").entered();

        let func: WasmFnTrampoline = self
            .instance
            .get_typed_func(&mut self.store, "__wasm_fn_trampoline")?;
        func.call(
            &mut self.store,
            (ptr.0, entity.map(|e| e.into_raw()).unwrap_or(0)),
        )
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

pub(crate) struct RunState {
    world: *const dyn WorldProvider,
    pub records: *const dyn RecordProvider,
    pub physics_pipeline: *const game_physics::Pipeline,
    effects: *mut Effects,
    dependencies: *mut Dependencies,
    next_entity_id: u64,
    next_inventory_id: u64,
    pub new_world: World,
    pub events: Vec<DispatchEvent>,
    pub host_buffers: Vec<Vec<u8>>,
}

// Make `RunState` `Send` + `Sync` to make `Executor` recursively `Send` + `Sync`.
// This is safe because we guarantee that the stored raw pointers are only used
// for the single invocation of `Executor::update`.
unsafe impl Send for RunState {}
unsafe impl Sync for RunState {}

impl RunState {
    pub fn new(
        world: *const dyn WorldProvider,
        physics_pipeline: *const game_physics::Pipeline,
        effects: *mut Effects,
        dependencies: *mut Dependencies,
        records: *const dyn RecordProvider,
        new_world: World,
        host_buffers: Vec<Vec<u8>>,
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
            events: Vec::new(),
            host_buffers,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
// TODO: Can be nonzero.
pub struct ErrorCode(u32);

impl ErrorCode {
    pub const NO_ENTITY: Self = Self(RESULT_NO_ENTITY);
    pub const NO_INVENTORY_SLOT: Self = Self(RESULT_NO_INVENTORY_SLOT);
    pub const NO_COMPONENT: Self = Self(RESULT_NO_COMPONENT);
}

impl ErrorCode {
    pub fn to_u32(self) -> u32 {
        self.0
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

    pub fn despawn(&mut self, id: EntityId) -> Result<(), ErrorCode> {
        if !self.new_world.contains(id) {
            return Err(ErrorCode::NO_ENTITY);
        }

        self.effects().push(Effect::EntityDespawn(id));
        self.new_world.despawn(id);
        Ok(())
    }

    pub fn get_component(
        &mut self,
        entity_id: EntityId,
        component_id: RecordReference,
    ) -> Result<&RawComponent, ErrorCode> {
        if !self.new_world.contains(entity_id) {
            return Err(ErrorCode::NO_ENTITY);
        }

        let Some(component) = self.new_world.get(entity_id, component_id) else {
            return Err(ErrorCode::NO_COMPONENT);
        };

        unsafe { &mut *self.dependencies }
            .push(Dependency::EntityComponent(entity_id, component_id));
        Ok(component)
    }

    pub fn insert_component(
        &mut self,
        entity_id: EntityId,
        id: RecordReference,
        component: RawComponent,
    ) -> Result<(), ErrorCode> {
        if !self.new_world.contains(entity_id) {
            return Err(ErrorCode::NO_ENTITY);
        }

        self.effects()
            .push(Effect::EntityComponentInsert(EntityComponentInsert {
                entity: entity_id,
                component_id: id,
                component: component.clone(),
            }));
        self.new_world.insert(entity_id, id, component);
        Ok(())
    }

    pub fn remove_component(
        &mut self,
        entity_id: EntityId,
        id: RecordReference,
    ) -> Result<(), ErrorCode> {
        if !self.new_world.contains(entity_id) {
            return Err(ErrorCode::NO_ENTITY);
        }

        if self.new_world.remove(entity_id, id).is_some() {
            self.effects()
                .push(Effect::EntityComponentRemove(EntityComponentRemove {
                    entity: entity_id,
                    component_id: id,
                }));
            Ok(())
        } else {
            Err(ErrorCode::NO_COMPONENT)
        }
    }

    fn reconstruct_inventory(&self, id: EntityId) -> Option<Inventory> {
        let mut inventory = unsafe { &*self.world }
            .inventory(id)
            .cloned()
            .unwrap_or_default();

        let mut is_some = unsafe { &*self.world }.inventory(id).is_some();

        for effect in unsafe { &*self.effects }.iter() {
            is_some = true;

            match effect {
                Effect::InventoryInsert(eid, slot_id, stack) if *eid == id => {
                    inventory.insert_at_slot(*slot_id, stack.clone()).unwrap();
                }
                Effect::InventoryRemove(eid, slot_id, quantity) if *eid == id => {
                    inventory.remove(slot_id, *quantity as u32);
                }
                Effect::InventoryClear(eid) if *eid == id => {
                    inventory.clear();
                }
                Effect::InventoryComponentInsert(eid, slot_id, comp_id, comp) if *eid == id => {
                    inventory
                        .get_mut(slot_id)
                        .unwrap()
                        .item
                        .components
                        .insert(*comp_id, comp.clone());
                }
                Effect::InventoryComponentRemove(eid, slot_id, comp_id) if *eid == id => {
                    inventory
                        .get_mut(slot_id)
                        .unwrap()
                        .item
                        .components
                        .remove(*comp_id);
                }
                _ => (),
            }
        }

        if is_some {
            Some(inventory)
        } else {
            None
        }
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
    ) -> Result<(), ErrorCode> {
        let Some(inventory) = unsafe { &*self.world }.inventory(entity) else {
            return Err(ErrorCode::NO_ENTITY);
        };

        if inventory.clone().remove(slot, quantity as u32).is_some() {
            self.effects()
                .push(Effect::InventoryRemove(entity, slot, quantity));
            Ok(())
        } else {
            Err(ErrorCode::NO_INVENTORY_SLOT)
        }
    }

    pub fn inventory_clear(&mut self, entity: EntityId) -> Result<(), ErrorCode> {
        if self.reconstruct_inventory(entity).is_none() {
            return Err(ErrorCode::NO_ENTITY);
        };

        self.effects().push(Effect::InventoryClear(entity));
        Ok(())
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
    ) -> Result<(), ErrorCode> {
        let Some(inventory) = self.reconstruct_inventory(entity) else {
            return Err(ErrorCode::NO_ENTITY);
        };

        if inventory.get(slot).is_none() {
            return Err(ErrorCode::NO_INVENTORY_SLOT);
        };

        self.effects().push(Effect::InventoryComponentInsert(
            entity,
            slot,
            component_id,
            component,
        ));
        Ok(())
    }

    pub fn inventory_component_remove(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        component_id: RecordReference,
    ) -> Result<(), ErrorCode> {
        let Some(inventory) = unsafe { &*self.world }.inventory(entity) else {
            return Err(ErrorCode::NO_ENTITY);
        };

        if inventory.get(slot).is_none() {
            return Err(ErrorCode::NO_INVENTORY_SLOT);
        };

        self.effects()
            .push(Effect::InventoryComponentRemove(entity, slot, component_id));
        Ok(())
    }

    pub fn inventory_set_equipped(
        &mut self,
        entity: EntityId,
        slot: InventorySlotId,
        equipped: bool,
    ) -> Result<(), ErrorCode> {
        let Some(inventory) = unsafe { &*self.world }.inventory(entity) else {
            return Err(ErrorCode::NO_ENTITY);
        };

        let Some(stack) = inventory.get(slot) else {
            return Err(ErrorCode::NO_INVENTORY_SLOT);
        };

        if stack.item.equipped == equipped {
            // Item is already in desired state, don't update
            // anything but we still succeed the request.
            Ok(())
        } else {
            self.effects()
                .push(Effect::InventoryItemUpdateEquip(entity, slot, equipped));
            Ok(())
        }
    }

    pub fn player_lookup(&mut self, entity_id: EntityId) -> Option<PlayerId> {
        unsafe { &*self.world }.player(entity_id)
    }

    pub fn player_set_active(
        &mut self,
        player: PlayerId,
        entity: EntityId,
    ) -> Result<(), ErrorCode> {
        if !self.new_world.contains(entity) {
            return Err(ErrorCode::NO_ENTITY);
        }

        self.effects()
            .push(Effect::PlayerSetActive(PlayerSetActive { player, entity }));
        Ok(())
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
