//! Game (dynamic) scripting

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

use std::collections::HashMap;
use std::path::Path;

use bevy_ecs::system::Resource;
use game_common::events::EventQueue;
use game_common::record::RecordReference;
use game_common::world::world::WorldViewMut;
use instance::ScriptInstance;
use script::Script;
use scripts::RecordTargets;
use slotmap::{DefaultKey, SlotMap};
use wasmtime::{Config, Engine};

pub mod abi;
pub mod actions;
pub mod events;
pub mod instance;
pub mod plugin;
pub mod script;
pub mod scripts;

mod builtin;

#[derive(Resource)]
pub struct ScriptServer {
    scripts: SlotMap<DefaultKey, Script>,
    engine: Engine,
}

impl ScriptServer {
    pub fn new() -> Self {
        let config = Config::new();

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

    pub fn get<'world, 'view>(
        &self,
        handle: &Handle,
        world: &'view mut WorldViewMut<'world>,
        physics_pipeline: &'view game_physics::Pipeline,
    ) -> Option<ScriptInstance<'world, 'view>> {
        let script = self.scripts.get(handle.id)?;

        Some(ScriptInstance::new(
            &self.engine,
            &script.module,
            script.events,
            world,
            physics_pipeline,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Handle {
    id: DefaultKey,
}

pub struct Context<'a, 'b> {
    pub view: &'a mut WorldViewMut<'b>,
    pub physics_pipeline: &'a game_physics::Pipeline,
    pub record_targets: &'a RecordTargets,
    pub events: &'a mut EventQueue,
    pub server: &'a ScriptServer,
}
