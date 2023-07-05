use ahash::HashMap;
use game_common::entity::EntityId;
use game_common::world::cell::square;
use game_common::world::entity::Entity;
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

    pub known_entities: KnownEntities,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            full_update: true,
            cells: Cells::new(CellId::new(0.0, 0.0, 0.0)),
            id: None,
            head: 0,
            known_entities: KnownEntities::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cells {
    /// The origin of the tracked entity.
    origin: CellId,
    cells: Vec<CellId>,
}

impl Cells {
    pub fn new(origin: CellId) -> Self {
        let cells = square(origin, 1);

        Self { origin, cells }
    }

    pub fn contains(&self, id: CellId) -> bool {
        self.origin == id || self.cells.contains(&id)
    }

    pub fn origin(&self) -> CellId {
        self.origin
    }

    pub fn set(&mut self, origin: CellId, distance: u32) -> UpdateCells {
        debug_assert_ne!(self.origin, origin);

        self.origin = origin;

        let new_cells = square(origin, distance);
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

    pub fn iter(&self) -> impl Iterator<Item = CellId> + '_ {
        self.cells().iter().copied()
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

/// Entities that client is aware of.
#[derive(Clone, Debug, Default)]
pub struct KnownEntities {
    pub entities: HashMap<EntityId, Entity>,
}

impl KnownEntities {
    pub fn new() -> Self {
        Self {
            entities: HashMap::default(),
        }
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    pub fn remove(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    pub fn contains(&mut self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }
}
