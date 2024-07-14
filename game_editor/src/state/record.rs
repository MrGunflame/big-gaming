use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use game_data::record::Record;
// use game_ui::reactive::{ReadSignal, WriteSignal};
use parking_lot::{Mutex, RwLock};

#[derive(Clone, Debug, Default)]
pub struct Records {
    next_id: Arc<RwLock<HashMap<ModuleId, u32>>>,
    records: Arc<RwLock<HashMap<RecordReference, Record>>>,
    // signal: Arc<Mutex<Option<WriteSignal<()>>>>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            records: Arc::default(),
            // signal: Arc::default(),
            next_id: Arc::default(),
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

        records
            .get(&RecordReference { module, record: id })
            .cloned()
    }

    pub fn insert(&self, module: ModuleId, record: Record) {
        let mut next_id = self.next_id.write();
        let val = next_id.entry(module).or_default();
        if record.id.0 >= *val {
            assert_ne!(*val, u32::MAX);
            *val += record.id.0 + 1;
        }

        let mut records = self.records.write();
        records.insert(
            RecordReference {
                module,
                record: record.id,
            },
            record,
        );

        // if let Some(signal) = &*self.signal.lock() {
        //     signal.wake();
        // }
    }

    pub fn take_id(&self, module: ModuleId) -> RecordId {
        let mut next_id = self.next_id.write();
        // Start with ID 1, we reserve ID 0 for future use.
        let val = next_id.entry(module).or_insert(1);
        assert_ne!(*val, u32::MAX);
        let id = RecordId(*val);
        *val += 1;
        id
    }

    pub fn update(&self, module: ModuleId, record: Record) {
        let mut records = self.records.write();
        if let Some(rec) = records.get_mut(&RecordReference {
            module,
            record: record.id,
        }) {
            *rec = record;
        }

        // if let Some(signal) = &*self.signal.lock() {
        //     signal.wake();
        // }
    }

    pub fn iter(&self) -> Iter<'_> {
        let records = self.records.read();
        let keys = records.keys().map(|k| *k).collect::<Vec<_>>().into_iter();
        drop(records);

        Iter { keys, inner: self }
    }

    // pub fn signal(&self, insert: impl FnOnce() -> WriteSignal<()>) -> ReadSignal<()> {
    //     let mut signal = self.signal.lock();
    //     match &*signal {
    //         Some(signal) => signal.subscribe(),
    //         None => {
    //             *signal = Some(insert());
    //             signal.as_ref().unwrap().subscribe()
    //         }
    //     }
    // }

    pub fn remove(&self, module: ModuleId, id: RecordId) {
        let mut records = self.records.write();
        records.remove(&RecordReference { module, record: id });

        // if let Some(signal) = &*self.signal.lock() {
        //     signal.wake();
        // }
    }
}

pub struct Iter<'a> {
    keys: std::vec::IntoIter<RecordReference>,
    inner: &'a Records,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (ModuleId, Record);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ref_) = self.keys.next() {
            if let Some(record) = self.inner.get(ref_.module, ref_.record) {
                return Some((ref_.module, record));
            }
        }

        None
    }
}
