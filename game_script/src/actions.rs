//! Action script handlers

use std::collections::HashMap;

use game_common::record::RecordId;

use crate::Handle;

#[derive(Clone, Debug)]
pub struct Actions {
    actions: HashMap<RecordId, Handle>,
}

impl Actions {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    pub fn insert(&mut self, record: RecordId, handle: Handle) {
        self.actions.insert(record, handle);
    }

    pub fn get(&self, record: RecordId) -> Option<Handle> {
        self.actions.get(&record).cloned()
    }

    pub fn remove(&mut self, record: RecordId) {
        self.actions.remove(&record);
    }
}
