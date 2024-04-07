//! Game (dynamic) scripting

use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};

use dependency::Dependencies;
use effect::Effects;
use events::DispatchEvent;
use game_common::entity::EntityId;
use game_common::events::{Event, EventQueue};
use game_common::record::RecordReference;
use game_common::world::World;
use game_data::record::Record;
use game_tracing::trace_span;
use game_wasm::encoding::{encode_fields, BinaryWriter};
use game_wasm::events::{PLAYER_CONNECT, PLAYER_DISCONNECT};
use game_wasm::player::PlayerId;
use instance::{HostBufferPool, InstancePool, RunState, State};
use script::{Script, ScriptLoadError};
use wasmtime::{Config, Engine, OptLevel, WasmBacktraceDetails};

pub mod effect;

mod builtin;
mod dependency;
mod events;
mod instance;
mod script;

pub struct Executor {
    engine: Engine,
    scripts: Vec<Script>,
    instances: InstancePool,
    systems: Vec<System>,
    action_handlers: HashMap<RecordReference, Vec<Entry>>,
    event_handlers: HashMap<RecordReference, Vec<Entry>>,
    // Reuse memory for invocations across `update` calls.
    invocations: VecDeque<Invocation>,
    host_buffer_pool: HostBufferPool,
}

impl Executor {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.wasm_backtrace(true);
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        config.cranelift_opt_level(OptLevel::SpeedAndSize);
        let engine = Engine::new(&config).unwrap();

        Self {
            instances: InstancePool::new(&engine),
            engine,
            scripts: Vec::new(),
            systems: vec![],
            action_handlers: HashMap::new(),
            event_handlers: HashMap::new(),
            invocations: VecDeque::with_capacity(32),
            host_buffer_pool: HostBufferPool::default(),
        }
    }

    pub fn load(&mut self, bytes: &[u8]) -> Result<Handle, ScriptLoadError> {
        let script = Script::new(bytes, &self.engine)?;

        let index = self.scripts.len();
        let handle = Handle(index);

        let state = self
            .instances
            .init(&self.engine, &script.module, handle)
            .map_err(ScriptLoadError::Init)?;

        self.systems.extend(state.systems);

        for (id, entries) in state.actions {
            self.action_handlers.entry(id).or_default().extend(entries);
        }

        for (id, entries) in state.event_handlers {
            self.event_handlers.entry(id).or_default().extend(entries);
        }

        self.scripts.push(script);
        Ok(handle)
    }

    pub fn update(&mut self, ctx: Context<'_>) -> Effects {
        let _span = trace_span!("Executor::update").entered();

        let world = ctx.world.world();

        for system in &self.systems {
            'entities: for entity in world.entities() {
                let components = world.components(entity);

                for component in &system.query.components {
                    if components.get(*component).is_none() {
                        continue 'entities;
                    }
                }

                self.invocations.push_back(Invocation {
                    script: system.script,
                    fn_ptr: system.ptr,
                    host_buffers: Vec::new(),
                    entity: Some(entity),
                });
            }
        }

        while let Some(event) = ctx.events.pop() {
            let (entries, action_buffer, entity) = match event {
                Event::Action(event) => match self.action_handlers.get(&event.action.0) {
                    Some(entries) => (entries, event.data, event.entity),
                    None => continue,
                },
                Event::PlayerConnect(event) => {
                    let (fields, data) = BinaryWriter::new().encoded(&event);
                    let fields = encode_fields(&fields);

                    self.schedule_event(DispatchEvent {
                        id: PLAYER_CONNECT,
                        data,
                        fields,
                    });
                    continue;
                }
                Event::PlayerDisconnect(event) => {
                    let (fields, data) = BinaryWriter::new().encoded(&event);
                    let fields = encode_fields(&fields);

                    self.schedule_event(DispatchEvent {
                        id: PLAYER_DISCONNECT,
                        data,
                        fields,
                    });
                    continue;
                }
                _ => continue,
            };

            let action_buffer = self.host_buffer_pool.insert(action_buffer);
            let empty_buffer = self.host_buffer_pool.insert(vec![]);

            for entry in entries {
                self.invocations.push_back(Invocation {
                    script: entry.script,
                    fn_ptr: entry.fn_ptr,
                    host_buffers: vec![action_buffer, empty_buffer],
                    entity: Some(entity),
                });
            }
        }

        let mut dependencies = Dependencies::default();
        let mut effects = Effects::default();

        // Reuse the same world so that dependant scripts don't overwrite
        // each other.
        // TODO: Still need to figure out what happens if scripts access the
        // same state.
        let mut state = RunState::new(
            ctx.world as *const dyn WorldProvider,
            ctx.physics,
            &mut effects,
            &mut dependencies,
            ctx.records as *const dyn RecordProvider,
            ctx.world.world().clone(),
            vec![],
            &self.host_buffer_pool,
        );

        // TODO: Right now if two event handlers call each other unconditionally we will
        // never stop scheduling more invocations and deadlock. We should implement some
        // sort of cycle checks and stop when an event schedules an event from which the
        // the event was dispatched from.

        while let Some(invocation) = self.invocations.pop_front() {
            state.host_buffers = invocation.host_buffers;

            let runnable = self.instances.get(State::Run(state), invocation.script);

            if let Err(err) = runnable.call(invocation.fn_ptr, invocation.entity) {
                tracing::error!("Error running script: {}", err);
            }

            state = runnable.into_state();

            for event in state.events.drain(..) {
                self.schedule_event(event);
            }
        }

        self.host_buffer_pool.clear();

        effects
    }

    fn schedule_event(&mut self, event: DispatchEvent) {
        tracing::debug!("scheduling event {:?}", event);

        let Some(handlers) = self.event_handlers.get(&event.id) else {
            return;
        };

        if handlers.is_empty() {
            return;
        }

        let data = self.host_buffer_pool.insert(event.data);
        let fields = self.host_buffer_pool.insert(event.fields);

        for handler in handlers {
            tracing::debug!("found handler for event {:?}: {:?}", event.id, handler);

            self.invocations.push_back(Invocation {
                script: handler.script,
                fn_ptr: handler.fn_ptr,
                host_buffers: vec![data, fields],
                entity: None,
            });
        }
    }
}

impl Debug for Executor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor")
            .field("scripts", &self.scripts)
            .field("instances", &self.instances)
            .field("systems", &self.systems)
            .field("action_handlers", &self.action_handlers)
            .field("event_handlers", &self.event_handlers)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
struct Invocation {
    script: Handle,
    fn_ptr: Pointer,
    host_buffers: Vec<usize>,
    entity: Option<EntityId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Handle(usize);

pub struct Context<'a> {
    pub world: &'a dyn WorldProvider,
    pub physics: &'a game_physics::Pipeline,
    pub events: &'a mut EventQueue,
    pub records: &'a dyn RecordProvider,
}

pub trait WorldProvider: 'static {
    fn world(&self) -> &World;
    fn player(&self, id: EntityId) -> Option<PlayerId>;
}

pub trait RecordProvider: 'static {
    fn get(&self, id: RecordReference) -> Option<&Record>;
}

#[derive(Clone, Debug)]
struct Entry {
    script: Handle,
    fn_ptr: Pointer,
}

#[derive(Clone, Debug)]
struct System {
    script: Handle,
    ptr: Pointer,
    query: SystemQuery,
}

#[derive(Clone, Debug)]
struct SystemQuery {
    components: Vec<RecordReference>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
struct Pointer(u32);

impl Debug for Pointer {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}
