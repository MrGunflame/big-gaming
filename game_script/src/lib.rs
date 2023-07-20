//! Game (dynamic) scripting

use bevy_ecs::system::Resource;
use game_common::world::world::WorldViewMut;
use instance::ScriptInstance;
use queue::CommandQueue;
use script::Script;
use wasmtime::{Config, Engine};

pub mod abi;
pub mod actions;
pub mod events;
pub mod host;
pub mod instance;
pub mod plugin;
pub mod queue;
pub mod script;
pub mod scripts;

mod builtin;

#[derive(Resource)]
pub struct ScriptServer {
    scripts: Vec<Script>,
    next_id: u64,
    engine: Engine,
}

impl ScriptServer {
    pub fn new() -> Self {
        let config = Config::new();

        Self {
            scripts: Vec::new(),
            next_id: 0,
            engine: Engine::new(&config).unwrap(),
        }
    }

    pub fn get<'world>(
        &self,
        handle: &Handle,
        world: WorldViewMut<'world>,
        queue: &'world mut CommandQueue,
        physics_pipeline: &'world game_physics::Pipeline,
    ) -> Option<ScriptInstance<'world>> {
        let script = self.scripts.get(handle.id as usize)?;

        match script {
            Script::Wasm(s) => Some(ScriptInstance::new(
                &self.engine,
                &s.module,
                s.events,
                world,
                queue,
                physics_pipeline,
            )),
        }
    }

    pub fn insert(&mut self, script: Script) -> Handle {
        let id = self.next_id;
        self.next_id += 1;

        self.scripts.push(script);

        Handle { id }
    }
}

#[derive(Clone, Debug)]
pub struct Handle {
    id: u64,
}
