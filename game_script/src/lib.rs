//! Game (dynamic) scripting

use std::fmt::Debug;
use std::path::Path;

use dependency::Dependencies;
use effect::Effects;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_common::record::RecordReference;
use game_common::world::entity::Entity;
use game_common::world::world::{WorldViewMut, WorldViewRef};
use game_data::record::Record;
use instance::ScriptInstance;
use script::Script;
use slotmap::{DefaultKey, SlotMap};
use wasmtime::{Config, Engine, WasmBacktraceDetails};

pub mod effect;
pub mod executor;
pub mod scripts;

mod abi;
mod builtin;
mod dependency;
mod events;
mod instance;
mod script;

pub struct ScriptServer {
    scripts: SlotMap<DefaultKey, Script>,
    engine: Engine,
}

impl ScriptServer {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.wasm_backtrace(true);
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);

        Self {
            scripts: SlotMap::new(),
            engine: Engine::new(&config).unwrap(),
        }
    }

    pub fn load<P>(&mut self, path: P) -> Result<Handle, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let script = Script::load(path.as_ref(), &self.engine)?;
        let id = self.scripts.insert(script);
        Ok(Handle { id })
    }

    pub fn get<'a>(
        &self,
        handle: &Handle,
        world: &'a dyn WorldProvider,
        physics_pipeline: &'a game_physics::Pipeline,
        effects: &'a mut Effects,
        dependencies: &'a mut Dependencies,
        records: &'a dyn RecordProvider,
    ) -> Option<ScriptInstance<'a>> {
        let script = self.scripts.get(handle.id)?;

        Some(ScriptInstance::new(
            &self.engine,
            &script.module,
            script.events,
            world,
            physics_pipeline,
            effects,
            dependencies,
            records,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Handle {
    id: DefaultKey,
}

pub struct Context<'a> {
    pub view: &'a dyn WorldProvider,
    pub physics_pipeline: &'a game_physics::Pipeline,
    pub events: &'a mut EventQueue,
    pub records: &'a dyn RecordProvider,
}

impl Debug for ScriptServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptServer").finish_non_exhaustive()
    }
}

pub trait WorldProvider {
    fn get(&self, id: EntityId) -> Option<&Entity>;
    fn inventory(&self, id: EntityId) -> Option<&Inventory>;
}

impl WorldProvider for WorldViewRef<'_> {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.get(id)
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }
}

impl WorldProvider for WorldViewMut<'_> {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.get(id)
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories().get(id)
    }
}

pub trait RecordProvider {
    fn get(&self, id: RecordReference) -> Option<&Record>;
}
