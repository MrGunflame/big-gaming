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

/// Returns all cells around center.
pub fn square(center: CellId, distance: u32) -> Vec<CellId> {
    debug_assert!(distance as i32 >= 0);

    let distance = distance as i32;
    let mut cells = vec![];

    let center = center.to_i32();
    for x in center.x - distance..=center.x + distance {
        for y in center.y - distance..=center.y + distance {
            for z in center.z - distance..=center.z + distance {
                cells.push(CellId::from_i32(IVec3::new(x, y, z)));
            }
        }
    }

    let num_cells_expected = (distance * 2 + 1).pow(3);
    debug_assert_eq!(cells.len(), usize::try_from(num_cells_expected).unwrap());

    cells
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use glam::IVec3;

    use super::{square, CellId};

    #[test]
    fn cell_id_square_0() {
        let center = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 0;

        let res = square(center, distance);
        match_cells_exact(&res, &[center]);
    }

    #[test]
    fn cell_id_square_0_non_zero() {
        let center = CellId::from_i32(IVec3::new(1, 2, 3));
        let distance = 0;

        let res = square(center, distance);
        match_cells_exact(&res, &[center]);
    }

    #[test]
    fn cell_id_square_1() {
        let center = CellId::from_i32(IVec3::new(0, 0, 0));
        let distance = 1;

        let res = square(center, distance);
        match_cells_exact(
            &res,
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
    fn match_cells_exact(lhs: &[CellId], rhs: &[CellId]) {
        let mut lhs: HashSet<_> = lhs.iter().copied().collect();

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
