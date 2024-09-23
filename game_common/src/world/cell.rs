use std::iter::FusedIterator;

use glam::IVec3;

use super::entity::Entity;
use super::world::{CellViewRef, WorldViewMut};

pub use game_wasm::cell::{CellId, CELL_SIZE, CELL_SIZE_UINT};

#[derive(Debug)]
pub struct Cell {
    id: CellId,
    entities: Vec<Entity>,
    #[cfg(debug_assertions)]
    loaded: bool,
}

impl Cell {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<CellId>,
    {
        Self {
            id: id.into(),
            entities: Vec::new(),
            #[cfg(debug_assertions)]
            loaded: false,
        }
    }

    pub fn id(&self) -> CellId {
        self.id
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn spawn<T>(&mut self, entity: T)
    where
        T: Into<Entity>,
    {
        self.entities.push(entity.into());
    }

    pub fn load(&mut self, view: &mut WorldViewMut<'_>) {
        #[cfg(debug_assertions)]
        {
            self.loaded = true;
        }

        for entity in &mut self.entities {
            entity.id = view.spawn(entity.clone());
        }
    }

    pub fn unload(&mut self, view: &mut WorldViewMut<'_>) {
        #[cfg(debug_assertions)]
        assert!(self.loaded, "attempted to unload cell before it was loaded");

        for entity in &self.entities {
            view.despawn(entity.id);
        }
    }

    pub fn update(&mut self, view: &CellViewRef<'_>) {
        #[cfg(debug_assertions)]
        assert!(self.loaded, "attempted to update cell before it was loaded");

        // Expect a similar amount of entities.
        let mut entities = Vec::with_capacity(self.entities.len());

        for entity in view.iter() {
            entities.push(entity.clone());
        }
    }
}

/// An `Iterator` yielding the cells in the cube around a center cell.
///
/// A distance value of `n` will yield all [`CellId`]s in the range of `center - n..=center + n` in
/// every direction. The `center` [`CellId`] is always returned (with a distance of `0`).
///
/// # Example
///
/// A distance value of 2 will yield these [`CellId`]s with these cells (in all 3 Axes):
///
/// ```text
/// +---+---+---+---+---+
/// | 2 | 2 | 2 | 2 | 2 |
/// +---+---+---+---+---+
/// | 2 | 1 | 1 | 1 | 2 |
/// +---+---+---+---+---+
/// | 2 | 1 | 0 | 1 | 2 |
/// +---+---+---+---+---+
/// | 2 | 1 | 1 | 1 | 2 |
/// +---+---+---+---+---+
/// | 2 | 2 | 2 | 2 | 2 |
/// +---+---+---+---+---+
/// ```
#[derive(Clone, Debug)]
pub struct CubeIter {
    center: CellId,
    distance: u32,
    index: u32,
}

impl CubeIter {
    /// Creates a new `CubeIter` yielding all [`CellId`]s around `center` with a distance of
    /// `distance`.
    ///
    /// Refer to [`Self`] for more details.
    pub fn new(center: CellId, distance: u32) -> Self {
        debug_assert!(distance as i32 >= 0);

        Self {
            center,
            distance,
            index: 0,
        }
    }
}

impl Iterator for CubeIter {
    type Item = CellId;

    fn next(&mut self) -> Option<Self::Item> {
        let len_axis = self.distance * 2 + 1;
        if self.index as usize >= self.len() {
            return None;
        }

        let x = self.index % len_axis;
        let y = self.index / len_axis % len_axis;
        let z = self.index / len_axis / len_axis;

        // Map `0..len` to `center - distance..=center + distance`.
        let center = self.center.to_i32();
        let x = (center.x - self.distance as i32) + x as i32;
        let y = (center.y - self.distance as i32) + y as i32;
        let z = (center.z - self.distance as i32) + z as i32;

        self.index += 1;
        Some(CellId::from_i32(IVec3::new(x, y, z)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for CubeIter {
    fn len(&self) -> usize {
        (self.distance * 2 + 1).pow(3) as usize
    }
}

impl FusedIterator for CubeIter {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use glam::IVec3;

    use super::{CellId, CubeIter};

    #[test]
    fn cell_id_cube_0() {
        let center = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 0;

        let res = CubeIter::new(center, distance);
        match_cells_exact(res, &[center]);
    }

    #[test]
    fn cell_id_cube_0_non_zero() {
        let center = CellId::from_i32(IVec3::new(1, 2, 3));
        let distance = 0;

        let res = CubeIter::new(center, distance);
        match_cells_exact(res, &[center]);
    }

    #[test]
    fn cell_id_cube_1() {
        let center = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 1;

        let res = CubeIter::new(center, distance);
        match_cells_exact(
            res,
            &[
                center,
                CellId::from_i32(IVec3::new(0, 0, 1)),
                CellId::from_i32(IVec3::new(0, 0, -1)),
                CellId::from_i32(IVec3::new(0, 1, 0)),
                CellId::from_i32(IVec3::new(0, 1, 1)),
                CellId::from_i32(IVec3::new(0, 1, -1)),
                CellId::from_i32(IVec3::new(0, -1, 0)),
                CellId::from_i32(IVec3::new(0, -1, 1)),
                CellId::from_i32(IVec3::new(0, -1, -1)),
                CellId::from_i32(IVec3::new(1, 0, 0)),
                CellId::from_i32(IVec3::new(1, 0, 1)),
                CellId::from_i32(IVec3::new(1, 0, -1)),
                CellId::from_i32(IVec3::new(1, 1, 0)),
                CellId::from_i32(IVec3::new(1, 1, 1)),
                CellId::from_i32(IVec3::new(1, 1, -1)),
                CellId::from_i32(IVec3::new(1, -1, 0)),
                CellId::from_i32(IVec3::new(1, -1, 1)),
                CellId::from_i32(IVec3::new(1, -1, -1)),
                CellId::from_i32(IVec3::new(-1, 0, 0)),
                CellId::from_i32(IVec3::new(-1, 0, 1)),
                CellId::from_i32(IVec3::new(-1, 0, -1)),
                CellId::from_i32(IVec3::new(-1, 1, 0)),
                CellId::from_i32(IVec3::new(-1, 1, 1)),
                CellId::from_i32(IVec3::new(-1, 1, -1)),
                CellId::from_i32(IVec3::new(-1, -1, 0)),
                CellId::from_i32(IVec3::new(-1, -1, 1)),
                CellId::from_i32(IVec3::new(-1, -1, -1)),
            ],
        );
    }

    #[track_caller]
    fn match_cells_exact<L>(lhs: L, rhs: &[CellId])
    where
        L: Iterator<Item = CellId>,
    {
        let mut lhs: HashSet<_> = lhs.collect();

        for id in rhs {
            if !lhs.remove(&id) {
                panic!("missing {:?} in lhs", id);
            }
        }

        for id in lhs {
            panic!("missing {:?} in rhs", id);
        }
    }
}
