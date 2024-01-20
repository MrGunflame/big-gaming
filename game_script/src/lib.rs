//! Game (dynamic) scripting

use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::path::Path;

use dependency::Dependencies;
use effect::Effects;
use events::DispatchEvent;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::events::{Event, EventQueue};
use game_common::record::RecordReference;
use game_common::world::world::{WorldViewMut, WorldViewRef};
use game_common::world::World;
use game_data::record::Record;
use game_tracing::trace_span;
use game_wasm::components::Encode;
use game_wasm::events::{PLAYER_CONNECT, PLAYER_DISCONNECT};
use game_wasm::player::PlayerId;
use instance::{InstancePool, RunState, State};
use script::Script;
use wasmtime::{Config, Engine, WasmBacktraceDetails};

pub mod effect;

mod abi;
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
}

impl Executor {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.wasm_backtrace(true);
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        let engine = Engine::new(&config).unwrap();

        Self {
            instances: InstancePool::new(&engine),
            engine,
            scripts: Vec::new(),
            systems: vec![],
            action_handlers: HashMap::new(),
            event_handlers: HashMap::new(),
        }
    }

    pub fn load<P>(&mut self, path: P) -> Result<Handle, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let script = Script::load(path.as_ref(), &self.engine)?;

        let index = self.scripts.len();
        let handle = Handle(index);

        let state = self.instances.init(&self.engine, &script.module, handle)?;

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

        let mut invocations = VecDeque::new();

        let world = ctx.world.world();

        for system in &self.systems {
            'entities: for entity in world.entities() {
                let components = world.components(entity);

                for component in &system.query.components {
                    if components.get(*component).is_none() {
                        continue 'entities;
                    }
                }

                invocations.push_back(Invocation {
                    script: system.script,
                    fn_ptr: system.ptr,
                    action_buffer: None,
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
                Event::PlayerConnect(player) => {
                    let mut buf = Vec::new();
                    player.encode(&mut buf);

                    self.schedule_event(
                        &mut invocations,
                        DispatchEvent {
                            id: PLAYER_CONNECT,
                            data: buf,
                        },
                    );
                    continue;
                }
                Event::PlayerDisconnect(player) => {
                    let mut buf = Vec::new();
                    player.encode(&mut buf);

                    self.schedule_event(
                        &mut invocations,
                        DispatchEvent {
                            id: PLAYER_DISCONNECT,
                            data: buf,
                        },
                    );
                    continue;
                }
                _ => continue,
            };

            for entry in entries {
                invocations.push_back(Invocation {
                    script: entry.script,
                    fn_ptr: entry.fn_ptr,
                    action_buffer: Some(action_buffer.clone()),
                    entity: Some(entity),
                });
            }
        }

        let mut effects = Effects::default();

        // Reuse the same world so that dependant scripts don't overwrite
        // each other.
        // TODO: Still need to figure out what happens if scripts access the
        // same state.
        let mut new_world = ctx.world.world().clone();

        // TODO: Right now if two event handlers call each other unconditionally we will
        // never stop scheduling more invocations and deadlock. We should implement some
        // sort of cycle checks and stop when an event schedules an event from which the
        // the event was dispatched from.

        while let Some(invocation) = invocations.pop_front() {
            let mut dependencies = Dependencies::default();
            let state = RunState::new(
                unsafe {
                    core::mem::transmute::<&dyn WorldProvider, *const dyn WorldProvider>(ctx.world)
                },
                ctx.physics,
                &mut effects,
                &mut dependencies,
                unsafe {
                    core::mem::transmute::<&dyn RecordProvider, *const dyn RecordProvider>(
                        ctx.records,
                    )
                },
                new_world,
                invocation.action_buffer,
            );

            let runnable = self.instances.get(State::Run(state), invocation.script);

            if let Err(err) = runnable.call(invocation.fn_ptr, invocation.entity) {
                tracing::error!("Error running script: {}", err);
            }

            let state = runnable.into_state();
            new_world = state.new_world;

            for event in state.events {
                self.schedule_event(&mut invocations, event);
            }
        }

        effects
    }

    fn schedule_event(&self, invocations: &mut VecDeque<Invocation>, event: DispatchEvent) {
        let Some(handlers) = self.event_handlers.get(&event.id) else {
            return;
        };

        for handler in handlers {
            invocations.push_back(Invocation {
                script: handler.script,
                fn_ptr: handler.fn_ptr,
                action_buffer: Some(event.data.clone()),
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
    action_buffer: Option<Vec<u8>>,
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

pub trait WorldProvider {
    fn world(&self) -> &World;
    fn inventory(&self, id: EntityId) -> Option<&Inventory>;
    fn player(&self, id: EntityId) -> Option<PlayerId>;
}

impl WorldProvider for WorldViewRef<'_> {
    fn world(&self) -> &World {
        todo!()
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }

    fn player(&self, id: EntityId) -> Option<PlayerId> {
        todo!()
    }
}

impl WorldProvider for WorldViewMut<'_> {
    fn world(&self) -> &World {
        todo!()
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }

    fn player(&self, id: EntityId) -> Option<PlayerId> {
        todo!()
    }
}

pub trait RecordProvider {
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
