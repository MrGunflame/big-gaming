use ahash::HashMap;

use super::snapshot::EntityChange;
use super::CellId;

/// A list of events of delta events.
#[derive(Clone, Debug, Default)]
pub struct DeltaQueue {
    cells: HashMap<CellId, Vec<EntityChange>>,
}

impl DeltaQueue {
    pub fn push(&mut self, cell: CellId, event: EntityChange) {
        let entry = self.cells.entry(cell).or_default();
        entry.push(event);
    }

    pub fn cell(&self, cell: CellId) -> Option<&[EntityChange]> {
        self.cells.get(&cell).map(|v| v.as_slice())
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }
}
