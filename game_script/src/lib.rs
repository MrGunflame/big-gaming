//! Game (dynamic) scripting

use script::Script;

pub mod host;
pub mod script;

#[derive(Debug)]
pub struct ScriptServer {
    scripts: Vec<Script>,
    next_id: u64,
}

impl ScriptServer {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            next_id: 0,
        }
    }

    pub fn get(&self, handle: &Handle) -> Option<&Script> {
        self.scripts.get(handle.id as usize)
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
