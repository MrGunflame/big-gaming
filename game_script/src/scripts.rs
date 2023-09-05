//! Scripts assigned to an entity.

use std::collections::HashMap;

use game_common::record::RecordReference;

use crate::Handle;

#[derive(Clone, Debug, Default)]
pub struct RecordTargets {
    pub(crate) scripts: HashMap<RecordReference, Vec<Handle>>,
    pub(crate) actions: HashMap<RecordReference, Vec<RecordReference>>,
}

impl RecordTargets {
    pub fn push_script(&mut self, record: RecordReference, handle: Handle) {
        self.scripts.entry(record).or_default().push(handle);
    }

    pub fn push_action(&mut self, record: RecordReference, action: RecordReference) {
        self.actions.entry(record).or_default().push(action);
    }
}
