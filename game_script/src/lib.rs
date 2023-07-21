//! Game (dynamic) scripting

use std::path::Path;

use bevy_ecs::system::Resource;
use game_common::world::world::WorldViewMut;
use instance::ScriptInstance;
use queue::CommandQueue;
use script::Script;
use slotmap::{DefaultKey, SlotMap};
use wasmtime::{Config, Engine};

pub mod abi;
pub mod actions;
pub mod events;
pub mod instance;
pub mod plugin;
pub mod queue;
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

    pub fn insert(&mut self) {}

    pub fn get<'world>(
        &self,
        handle: &Handle,
        world: WorldViewMut<'world>,
        queue: &'world mut CommandQueue,
        physics_pipeline: &'world game_physics::Pipeline,
    ) -> Option<ScriptInstance<'world>> {
        let script = self.scripts.get(handle.id)?;

        Some(ScriptInstance::new(
            &self.engine,
            &script.module,
            script.events,
            world,
            queue,
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
}
