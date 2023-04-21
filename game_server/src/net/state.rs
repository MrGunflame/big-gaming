use bevy::prelude::IVec3;
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
        let cells = cell_distance(origin, 1);

        Self { origin, cells }
    }

    pub fn contains(&self, id: CellId) -> bool {
        self.origin == id || self.cells.contains(&id)
    }

    pub fn origin(&self) -> CellId {
        self.origin
    }

    pub fn set(&mut self, origin: CellId) -> UpdateCells {
        debug_assert_ne!(self.origin, origin);

        self.origin = origin;

        let new_cells = cell_distance(origin, 1);
        let old_cells = &self.cells;

        let mut loaded = vec![];
        let mut unloaded = vec![];

        for id in &new_cells {
            if !old_cells.contains(&id) {
                loaded.push(*id);
            }
        }

        for id in old_cells {
            if !new_cells.contains(&id) {
                unloaded.push(*id);
            }
        }

        self.cells = new_cells;

        UpdateCells { loaded, unloaded }
    }

    pub fn cells(&self) -> &[CellId] {
        &self.cells
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
fn cell_distance(origin: CellId, distance: u32) -> Vec<CellId> {
    let mut cells = vec![origin];

    let orig = origin.to_i32();

    let len = distance as i32;

    // X
    for i in 0..len {
        cells.push(CellId::from_i32(IVec3::new(orig.x + i + 1, orig.y, orig.z)));
        cells.push(CellId::from_i32(IVec3::new(orig.x - i - 1, orig.y, orig.z)));
    }

    // Y
    for i in 0..len {
        cells.push(CellId::from_i32(IVec3::new(orig.x, orig.y + i + 1, orig.z)));
        cells.push(CellId::from_i32(IVec3::new(orig.x, orig.y - i - 1, orig.z)));
    }

    // Z
    for i in 0..len {
        cells.push(CellId::from_i32(IVec3::new(orig.x, orig.y, orig.z + i + 1)));
        cells.push(CellId::from_i32(IVec3::new(orig.x, orig.y, orig.z - i - 1)));
    }

    cells
}

#[cfg(test)]
mod tests {
    use bevy::prelude::IVec3;
    use game_common::world::CellId;

    use super::cell_distance;

    #[test]
    fn test_distance_0() {
        let origin = CellId::new(0.0, 0.0, 0.0);
        let distance = 0;

        let cells = [origin];

        let res = cell_distance(origin, distance);
        assert_cells(&res, &cells);
    }

    #[test]
    fn test_distance_1() {
        let origin = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 1;

        let cells = [
            origin,
            // X
            CellId::from_i32(IVec3::new(1, 0, 0)),
            CellId::from_i32(IVec3::new(-1, 0, 0)),
            // Y
            CellId::from_i32(IVec3::new(0, 1, 0)),
            CellId::from_i32(IVec3::new(0, -1, 0)),
            // Z
            CellId::from_i32(IVec3::new(0, 0, 1)),
            CellId::from_i32(IVec3::new(0, 0, -1)),
        ];

        let res = cell_distance(origin, distance);
        assert_cells(&res, &cells);
    }

    /// Asserts the cells ignoring order.
    fn assert_cells(lhs: &[CellId], rhs: &[CellId]) {
        let mut lhs = lhs.to_owned();
        let mut rhs = rhs.to_owned();

        while !lhs.is_empty() && !rhs.is_empty() {
            let left = lhs[0];

            if let Some((index, _)) = rhs.iter().enumerate().find(|(_, right)| left == **right) {
                rhs.remove(index);
                lhs.remove(0);
            } else {
                dbg!(&lhs);
                dbg!(&rhs);

                panic!(
                    "found cell that was not expected: {:?} (expected one of {:?})",
                    left, rhs
                );
            }
        }

        assert!(
            lhs.is_empty(),
            "returned more cells than expected: {:?}",
            lhs
        );
        assert!(rhs.is_empty(), "missing expected cells: {:?}", rhs);
    }
}
