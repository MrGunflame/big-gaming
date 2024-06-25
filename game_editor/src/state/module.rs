//! The module state as in memory.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use game_common::module::{Module, ModuleId};
use game_ui::widgets::Callback;
use parking_lot::{Mutex, RwLock};

use super::capabilities::Capabilities;

#[derive(Clone, Debug, Default)]
pub struct Modules {
    modules: Arc<RwLock<HashMap<ModuleId, EditorModule>>>,
    // FIXME: What about multiple listeners?
    on_change: Arc<Mutex<Callback<()>>>,
}

impl Modules {
    pub fn new() -> Self {
        Self {
            modules: Arc::default(),
            on_change: Arc::default(),
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
        {
            let mut modules = self.modules.write();
            modules.insert(module.module.id, module);
        }

        self.on_change.lock().clone().call(());
    }

    pub fn get(&self, id: ModuleId) -> Option<EditorModule> {
        let modules = self.modules.write();
        modules.get(&id).cloned()
    }

    pub fn remove(&self, id: ModuleId) {
        {
            let mut modules = self.modules.write();
            modules.remove(&id);
        }

        self.on_change.lock().clone().call(());
    }

    pub fn iter(&self) -> ModuleIter<'_> {
        let modules = self.modules.read();
        let keys = modules.keys().copied().collect::<Vec<_>>().into_iter();

        ModuleIter { inner: self, keys }
    }

    pub fn set_on_change(&self, cb: Callback<()>) {
        *self.on_change.lock() = cb;
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
