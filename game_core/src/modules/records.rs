use std::collections::HashMap;

use bevy_ecs::system::Resource;
use game_common::record::RecordId;
use game_data::record::Record;
use game_script::events::Events;
use game_script::Handle;

#[derive(Clone, Debug, Resource)]
pub struct Records {
    records: HashMap<RecordId, Record>,
    /// Scripts assigned to records.
    scripts: HashMap<RecordId, Vec<ScriptRecord>>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            scripts: HashMap::new(),
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

    pub fn get_scripts(&self, id: RecordId) -> Option<&[ScriptRecord]> {
        self.scripts.get(&id).map(|s| s.as_slice())
    }

    pub fn insert_scripts(&mut self, id: RecordId, scripts: Vec<ScriptRecord>) {
        self.scripts.insert(id, scripts);
    }
}

#[derive(Clone, Debug)]
pub struct ScriptRecord {
    /// The handle to the script within the [`ScriptServer`].
    pub handle: Handle,
    /// The events that this script has listeners for.
    pub events: Events,
}
