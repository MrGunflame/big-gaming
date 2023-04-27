use std::collections::HashMap;

use bevy::prelude::Resource;
use game_common::module::ModuleId;
use game_common::record::RecordId;
use game_data::record::Record;

#[derive(Clone, Debug, Resource)]
pub struct Records {
    records: HashMap<(ModuleId, RecordId), Record>,
    next_id: u32,
}

impl Records {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),

            next_id: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn get(&self, module: ModuleId, id: RecordId) -> Option<&Record> {
        self.records.get(&(module, id))
    }

    pub fn insert(&mut self, module: ModuleId, record: Record) {
        if record.id.0 >= self.next_id {
            self.next_id = record.id.0.checked_add(1).unwrap();
        }

        self.records.insert((module, record.id), record);
    }

    pub fn push(&mut self, module: ModuleId, mut record: Record) -> RecordId {
        let id = RecordId(self.next_id);
        self.next_id = self.next_id.checked_add(1).unwrap();

        record.id = id;
        self.records.insert((module, id), record);

        id
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.records.iter(),
        }
    }
}

pub struct Iter<'a> {
    iter: std::collections::hash_map::Iter<'a, (ModuleId, RecordId), Record>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (ModuleId, &'a Record);

    fn next(&mut self) -> Option<Self::Item> {
        let ((id, _), record) = self.iter.next()?;
        Some((*id, record))
    }
}
