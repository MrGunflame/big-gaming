//! Game (dynamic) scripting

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::path::Path;

use dependency::Dependencies;
use effect::Effects;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::events::{Event, EventQueue};
use game_common::record::RecordReference;
use game_common::world::world::{WorldViewMut, WorldViewRef};
use game_common::world::World;
use game_data::record::Record;
use game_tracing::trace_span;
use game_wasm::player::PlayerId;
use instance::{InitState, InstancePool, RunState, State};
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
    /// Maps which components fire which scripts.
    targets: HashMap<RecordReference, Vec<Handle>>,
    instances: InstancePool,
    systems: Vec<System>,
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
            targets: HashMap::new(),
            scripts: Vec::new(),
            systems: vec![],
        }
    }

    pub fn load<P>(&mut self, path: P) -> Result<Handle, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let script = Script::load(path.as_ref(), &self.engine)?;

        let index = self.scripts.len();
        let handle = Handle(index);

        let mut instance = self.instances.get(
            &self.engine,
            &script.module,
            State::Init(InitState {
                script: handle,
                systems: vec![],
            }),
        );
        instance.init()?;

        let state = match instance.into_state() {
            State::Init(s) => s,
            _ => unreachable!(),
        };

        self.systems.extend(state.systems);

        self.scripts.push(script);
        Ok(handle)
    }

    pub fn register_script(&mut self, component: RecordReference, script: Handle) {
        let entry = self.targets.entry(component).or_default();

        // Registering a script on a component multiple times makes no
        // difference in execution behavior and is likely a mistake.
        debug_assert!(!entry.contains(&script));

        entry.push(script);
    }

    pub fn update(&mut self, ctx: Context<'_>) -> Effects {
        let _span = trace_span!("Executor::update").entered();

        let mut invocations = Vec::new();

        while let Some(event) = ctx.events.pop() {
            let (handles, action_buffer) = match &event {
                // An action is a special case. The script is not registered on
                // the component, but on the action record directly. We should only
                // call script for the exact triggered action, not any other.
                Event::Action(event) => match self.targets.get(&event.action.0) {
                    Some(handles) => (handles.clone(), Some(event.data.clone())),
                    // There are no handlers registered for the action. We should
                    // discard the action and pretend it was never called.
                    None => {
                        tracing::warn!(
                            "action {:?} queued, but there are no handlers for it",
                            event.action
                        );
                        continue;
                    }
                },
                Event::Collision(event) => (
                    self.fetch_components_scripts(event.entity, ctx.world.world()),
                    None,
                ),
                Event::Equip(event) => (
                    self.fetch_components_scripts(event.entity, ctx.world.world()),
                    None,
                ),
                Event::Unequip(event) => (
                    self.fetch_components_scripts(event.entity, ctx.world.world()),
                    None,
                ),
                Event::Update(entity) => (
                    self.fetch_components_scripts(*entity, ctx.world.world()),
                    None,
                ),
            };

            for handle in handles {
                invocations.push(Invocation {
                    event: event.clone(),
                    script: handle,
                    action_buffer: action_buffer.clone(),
                });
            }
        }

        let mut effects = Effects::default();

        // Reuse the same world so that dependant scripts don't overwrite
        // each other.
        // TODO: Still need to figure out what happens if scripts access the
        // same state.
        let mut new_world = ctx.world.world().clone();

        for invocation in invocations {
            let script = &self.scripts[invocation.script.0];

            let mut dependencies = Dependencies::default();
            let state = RunState::new(
                ctx.world,
                ctx.physics,
                &mut effects,
                &mut dependencies,
                ctx.records,
                new_world,
                invocation.action_buffer,
            );

            let mut runnable = self
                .instances
                .get(&self.engine, &script.module, State::Run(state));

            if let Err(err) = runnable.run(&invocation.event) {
                tracing::error!("Error running script: {}", err);
            }

            let state = runnable.into_state();
            new_world = match state {
                State::Run(s) => s.new_world,
                _ => unreachable!(),
            };
        }

        effects
    }

    fn fetch_components_scripts(&self, entity: EntityId, world: &World) -> Vec<Handle> {
        let components = world.components(entity);

        let mut scripts = Vec::new();
        for (id, _) in components.iter() {
            if let Some(handles) = self.targets.get(&id) {
                scripts.extend(handles);
            }
        }

        scripts.dedup();
        scripts
    }
}

impl Debug for Executor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor")
            .field("scripts", &self.scripts)
            .field("targets", &self.targets)
            .field("instances", &self.instances)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
struct Invocation {
    event: Event,
    script: Handle,
    action_buffer: Option<Vec<u8>>,
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
struct System {
    script: Handle,
    ptr: u32,
    query: SystemQuery,
}

#[derive(Clone, Debug)]
struct SystemQuery {
    components: Vec<RecordReference>,
}
