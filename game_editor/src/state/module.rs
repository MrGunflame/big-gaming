//! The module state as in memory.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};

use game_data::record::{Record, RecordId};
use parking_lot::RwLock;

#[derive(Clone, Debug)]
pub struct ModuleState {}

/// The records of a module.
#[derive(Clone, Debug, Default)]
pub struct Records {
    records: Arc<RwLock<HashMap<RecordId, Record>>>,
    next_id: Arc<AtomicU32>,
}

impl Records {
    pub fn get(&self, id: RecordId) -> Option<Record> {
        let inner = self.records.read();
        inner.get(&id).cloned()
    }

    pub fn put(&self, record: Record) {
        let mut inner = self.records.write();
        inner.insert(record.id, record);
    }

    pub fn insert(&self, mut record: Record) -> RecordId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        assert!(id != u32::MAX);
        record.id = RecordId(id);

        let mut inner = self.records.write();
        inner.insert(record.id, record);
        RecordId(id)
    }

    pub fn iter(&self) -> Iter<'_> {
        let inner = self.records.read();
        Iter {
            inner: self,
            keys: inner.keys().copied().collect(),
            index: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    inner: &'a Records,
    keys: Vec<RecordId>,
    index: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys.get(self.index)?;
        self.index += 1;
        self.inner.get(*key)
    }
}
