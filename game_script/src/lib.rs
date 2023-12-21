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
use instance::{InstancePool, State};
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
}

impl Executor {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.wasm_backtrace(true);
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        let engine = Engine::new(&config).unwrap();

        Self {
            engine,
            targets: HashMap::new(),
            scripts: Vec::new(),
            instances: InstancePool::new(),
        }
    }

    pub fn load<P>(&mut self, path: P) -> Result<Handle, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let script = Script::load(path.as_ref(), &self.engine)?;
        let index = self.scripts.len();
        self.scripts.push(script);
        Ok(Handle(index))
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
            let handles = match event {
                Event::Action(event) => {
                    self.fetch_components_scripts(event.entity, ctx.world.world())
                }
                _ => todo!(),
            };

            for handle in handles {
                invocations.push(Invocation {
                    event: event.clone(),
                    script: handle,
                });
            }
        }

        let mut effects = Effects::default();

        for invocation in invocations {
            let mut dependencies = Dependencies::default();

            let state = State::new(
                ctx.world,
                ctx.physics,
                &mut effects,
                &mut dependencies,
                ctx.records,
            );
            let script = &self.scripts[invocation.script.0];

            let mut runnable =
                self.instances
                    .get(&self.engine, invocation.script, &script.module, state);

            if let Err(err) = runnable.run(&invocation.event) {
                tracing::error!("Error running script: {}", err);
            }
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

struct Invocation {
    event: Event,
    script: Handle,
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
}

impl WorldProvider for WorldViewRef<'_> {
    fn world(&self) -> &World {
        todo!()
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }
}

impl WorldProvider for WorldViewMut<'_> {
    fn world(&self) -> &World {
        todo!()
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }
}

pub trait RecordProvider {
    fn get(&self, id: RecordReference) -> Option<&Record>;
}
