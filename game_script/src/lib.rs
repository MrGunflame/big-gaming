//! Game (dynamic) scripting

use game_common::world::world::WorldState;
use instance::ScriptInstance;
use script::Script;
use wasmtime::{Config, Engine};

pub mod host;
pub mod instance;
pub mod script;

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

    pub fn get<'a>(
        &self,
        handle: &Handle,
        world: &'a mut WorldState,
    ) -> Option<ScriptInstance<'a>> {
        let script = self.scripts.get(handle.id as usize)?;

        match script {
            Script::Wasm(s) => Some(ScriptInstance::new(&self.engine, &s.module, world)),
            _ => todo!(),
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
