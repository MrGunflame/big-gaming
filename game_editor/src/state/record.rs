use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use bevy_ecs::system::Resource;
use game_common::module::ModuleId;
use game_common::record::RecordId;
use game_data::record::Record;
use parking_lot::RwLock;

#[derive(Clone, Debug, Resource)]
pub struct Records {
    records: Arc<RwLock<HashMap<(ModuleId, RecordId), Record>>>,
    next_id: Arc<AtomicU32>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            records: Arc::default(),

            next_id: Arc::new(AtomicU32::new(1)),
        }
    }

    pub fn len(&self) -> usize {
        let records = self.records.read();
        records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, module: ModuleId, id: RecordId) -> Option<Record> {
        let records = self.records.read();

        records.get(&(module, id)).cloned()
    }

    pub fn insert(&self, module: ModuleId, mut record: Record) -> RecordId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        assert_ne!(id, 0, "record id overflow");

        record.id = RecordId(id);

        let mut records = self.records.write();
        records.insert((module, record.id), record);

        RecordId(id)
    }

    pub fn update(&self, module: ModuleId, record: Record) {
        let mut records = self.records.write();
        if let Some(rec) = records.get_mut(&(module, record.id)) {
            *rec = record;
        }
    }

    pub fn iter(&self) -> Iter<'_> {
        let records = self.records.read();
        let keys = records.keys().map(|k| *k).collect::<Vec<_>>().into_iter();
        drop(records);

        Iter { keys, inner: self }
    }
}

pub struct Iter<'a> {
    keys: std::vec::IntoIter<(ModuleId, RecordId)>,
    inner: &'a Records,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (ModuleId, Record);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((module_id, record_id)) = self.keys.next() {
            if let Some(record) = self.inner.get(module_id, record_id) {
                return Some((module_id, record));
            }
        }

        None
    }
}
