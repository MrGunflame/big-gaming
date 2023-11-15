//! Scripts assigned to an entity.

use std::collections::HashMap;

use game_common::record::RecordReference;

use crate::Handle;

#[derive(Clone, Debug, Default)]
pub struct RecordTargets {
    /// Scripts attached to records.
    pub(crate) scripts: HashMap<RecordReference, Vec<Handle>>,
}

impl RecordTargets {
    pub fn push(&mut self, id: RecordReference, handle: Handle) {
        self.scripts.entry(id).or_default().push(handle);
    }
}
