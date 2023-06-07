//! The module state as in memory.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::system::Resource;
use game_common::module::{Module, ModuleId};
use game_ui::reactive::{ReadSignal, WriteSignal};
use parking_lot::{Mutex, RwLock};

use super::capabilities::Capabilities;

#[derive(Clone, Debug, Resource)]
pub struct Modules {
    modules: Arc<RwLock<HashMap<ModuleId, EditorModule>>>,
    signal: Arc<Mutex<Option<WriteSignal<()>>>>,
}

impl Modules {
    pub fn new() -> Self {
        Self {
            modules: Arc::default(),
            signal: Arc::new(Mutex::new(None)),
        }
    }

    pub fn len(&self) -> usize {
        let modules = self.modules.read();
        modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&self, module: EditorModule) {
        let mut modules = self.modules.write();
        modules.insert(module.module.id, module);

        if let Some(signal) = &*self.signal.lock() {
            signal.wake();
        }
    }

    pub fn get(&self, id: ModuleId) -> Option<EditorModule> {
        let modules = self.modules.write();
        modules.get(&id).cloned()
    }

    pub fn remove(&self, id: ModuleId) {
        let mut modules = self.modules.write();
        modules.remove(&id);

        if let Some(signal) = &*self.signal.lock() {
            signal.wake();
        }
    }

    pub fn iter(&self) -> ModuleIter<'_> {
        let modules = self.modules.read();
        let keys = modules.keys().copied().collect::<Vec<_>>().into_iter();

        ModuleIter { inner: self, keys }
    }

    pub fn signal(&self, insert: impl FnOnce() -> WriteSignal<()>) -> ReadSignal<()> {
        let mut signal = self.signal.lock();
        match &*signal {
            Some(signal) => signal.subscribe(),
            None => {
                *signal = Some(insert());
                signal.as_ref().unwrap().subscribe()
            }
        }
    }
}

pub struct ModuleIter<'a> {
    inner: &'a Modules,
    keys: std::vec::IntoIter<ModuleId>,
}

impl<'a> Iterator for ModuleIter<'a> {
    type Item = EditorModule;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(key) = self.keys.next() {
            if let Some(val) = self.inner.get(key) {
                return Some(val);
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub struct EditorModule {
    pub module: Module,
    pub path: Option<PathBuf>,
    pub capabilities: Capabilities,
}
