use game_common::entity::EntityId;
use game_common::world::CellId;

#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub full_update: bool,
    /// Cells loaded by the peer.
    pub cells: Cells,
    /// The entity that is the host.
    pub id: Option<EntityId>,
    /// The snapshot index that the client's view is located at (currently modified).
    ///
    /// `head - 1..head` is the delta period.
    pub head: usize,
}

#[derive(Clone, Debug)]
pub struct Cells {
    /// The origin of the tracked entity.
    origin: CellId,
    cells: Vec<CellId>,
}

impl Cells {
    pub fn new(origin: CellId) -> Self {
        Self {
            origin,
            cells: Vec::new(),
        }
    }

    pub fn contains(&self, id: CellId) -> bool {
        self.origin == id || self.cells.contains(&id)
    }

    pub fn set(&mut self, origin: CellId) -> UpdateCells {
        debug_assert_ne!(self.origin, origin);

        let old_origin = std::mem::replace(&mut self.origin, origin);

        UpdateCells {
            loaded: vec![origin],
            unloaded: vec![old_origin],
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdateCells {
    loaded: Vec<CellId>,
    unloaded: Vec<CellId>,
}

impl UpdateCells {
    pub fn loaded<'a>(&'a self) -> impl Iterator<Item = CellId> + 'a {
        self.loaded.iter().copied()
    }

    pub fn unloaded<'a>(&'a self) -> impl Iterator<Item = CellId> + 'a {
        self.unloaded.iter().copied()
    }
}

/// Returns all cells within the given distance.
fn cell_distance(origin: CellId, distance: f32) -> Vec<CellId> {
    let mut cells = vec![origin];

    cells
}

#[cfg(test)]
mod tests {
    use game_common::world::CellId;

    use super::cell_distance;

    #[test]
    fn test_distance_0() {
        let origin = CellId::new(0.0, 0.0, 0.0);
        let distance = 0.0;

        assert_eq!(cell_distance(origin, distance), [origin])
    }
}
