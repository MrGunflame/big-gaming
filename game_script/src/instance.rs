use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use game_common::components::components::RawComponent;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_tracing::trace_span;
use game_wasm::player::PlayerId;
use game_wasm::raw::{RESULT_NO_COMPONENT, RESULT_NO_ENTITY};
use game_wasm::resource::RuntimeResourceId;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::builtin::register_host_fns;
use crate::effect::{
    CreateResource, DestroyResource, Effect, Effects, EntityComponentInsert, EntityComponentRemove,
    PlayerSetActive, UpdateResource,
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

    pub fn get(&mut self, state: State, handle: Handle) -> &mut Runnable {
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
    next_entity_id: u64,
    pub new_world: World,
    pub events: Vec<DispatchEvent>,
    pub host_buffers: Vec<usize>,
    host_buffer_pool: *const HostBufferPool,
    next_resource_id: u64,
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
        records: *const dyn RecordProvider,
        new_world: World,
        host_buffers: Vec<usize>,
        host_buffer_pool: *const HostBufferPool,
    ) -> Self {
        Self {
            world,
            physics_pipeline,
            effects,
            next_entity_id: 0,
            records,
            new_world,
            events: Vec::new(),
            host_buffers,
            host_buffer_pool,
            next_resource_id: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
// TODO: Can be nonzero.
pub struct ErrorCode(u32);

impl ErrorCode {
    pub const NO_ENTITY: Self = Self(RESULT_NO_ENTITY);
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

        if let Some(old_component) = self.new_world.get(entity_id, id) {
            // No need to actually do anything if the component didn't change.
            // This allows us to not trigger an `Effect` for this update, allowing
            // downstream code to be more efficient.
            if *old_component == component {
                return Ok(());
            }
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

    pub fn get_host_buffer(&self, key: u32) -> Option<&[u8]> {
        let index = *self.host_buffers.get(key as usize)?;
        unsafe { &*self.host_buffer_pool }.get(index)
    }

    pub fn insert_resource(&mut self, data: Arc<[u8]>) -> RuntimeResourceId {
        let id = self.allocate_temporary_resource_id();
        self.effects().push(Effect::CreateResource(CreateResource {
            id,
            data: data.clone(),
        }));
        self.new_world.insert_resource_with_id(data, id);
        id
    }

    pub fn update_resource(&mut self, id: RuntimeResourceId, data: Arc<[u8]>) -> bool {
        if self.new_world.get_resource(id).is_none() {
            return false;
        }

        self.new_world.insert_resource_with_id(data.clone(), id);
        self.effects()
            .push(Effect::UpdateResource(UpdateResource { id, data }));
        true
    }

    pub fn destroy_resource(&mut self, id: RuntimeResourceId) -> bool {
        if self.new_world.get_resource(id).is_none() {
            return false;
        }

        self.new_world.remove_resource(id);
        self.effects()
            .push(Effect::DestroyResource(DestroyResource { id }));
        true
    }

    pub fn get_resource_runtime(&mut self, id: RuntimeResourceId) -> Option<&[u8]> {
        self.new_world.get_resource(id)
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
        // If this overflows we have created more than 2**63 entities.
        // This basically means we're fucked.
        self.next_entity_id = self.next_entity_id.checked_add(1).unwrap();
        EntityId::from_raw(bits)
    }

    fn allocate_temporary_resource_id(&mut self) -> RuntimeResourceId {
        let bits = self.next_resource_id | (1 << 63);
        self.next_resource_id = self.next_resource_id.checked_add(1).unwrap();
        RuntimeResourceId::from_bits(bits)
    }
}

#[derive(Clone, Debug, Default)]
pub struct HostBufferPool {
    buffers: Vec<Vec<u8>>,
}

impl HostBufferPool {
    pub fn get(&self, index: usize) -> Option<&[u8]> {
        self.buffers.get(index).map(Vec::as_slice)
    }

    pub fn insert(&mut self, buf: Vec<u8>) -> usize {
        let index = self.buffers.len();
        self.buffers.push(buf);
        index
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}
