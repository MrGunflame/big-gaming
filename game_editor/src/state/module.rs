//! The module state as in memory.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::Resource;
use game_common::module::{Module, ModuleId};

use super::capabilities::Capabilities;

#[derive(Clone, Debug, Resource)]
pub struct Modules {
    modules: HashMap<ModuleId, EditorModule>,
}

impl Modules {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn insert(&mut self, module: EditorModule) {
        self.modules.insert(module.module.id, module);
    }

    pub fn get(&self, id: ModuleId) -> Option<&EditorModule> {
        self.modules.get(&id)
    }

    pub fn remove(&mut self, id: ModuleId) {
        self.modules.remove(&id);
    }

    pub fn iter(&self) -> ModuleIter<'_> {
        ModuleIter {
            iter: self.modules.values(),
        }
    }
}

pub struct ModuleIter<'a> {
    iter: std::collections::hash_map::Values<'a, ModuleId, EditorModule>,
}

impl<'a> Iterator for ModuleIter<'a> {
    type Item = &'a EditorModule;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Clone, Debug)]
pub struct EditorModule {
    pub module: Module,
    pub path: PathBuf,
    pub capabilities: Capabilities,
}
