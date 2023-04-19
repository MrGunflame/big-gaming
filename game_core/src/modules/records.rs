use std::collections::HashMap;

use bevy::prelude::Resource;
use game_data::record::{Record, RecordId};

#[derive(Clone, Debug, Resource)]
pub struct Records {
    records: HashMap<RecordId, Record>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn get(&self, id: RecordId) -> Option<&Record> {
        self.records.get(&id)
    }

    pub fn insert(&mut self, record: Record) {
        self.records.insert(record.id, record);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.records.values()
    }
}
