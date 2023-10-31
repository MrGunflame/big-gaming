use ahash::HashMap;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::world::cell::square;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::Entity;
use game_common::world::CellId;

use super::entities::Entities;

#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub full_update: bool,
    /// Cells loaded by the peer.
    pub cells: Cells,
    /// The entity that is the host.
    pub host: HostState,
    /// The snapshot index that the client's view is located at (currently modified).
    pub client_cf: ControlFrame,

    pub known_entities: KnownEntities,

    /// Constant interpolation buffer/delay of the peer.
    pub peer_delay: ControlFrame,

    pub entities: Entities,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            full_update: true,
            cells: Cells::new(CellId::new(0.0, 0.0, 0.0)),
            host: HostState::default(),
            client_cf: ControlFrame(0),
            known_entities: KnownEntities::new(),
            peer_delay: ControlFrame(0),
            entities: Entities::new(),
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

    pub fn set(&mut self, origin: CellId, distance: u32) {
        self.origin = origin;

        self.cells = square(origin, distance);
    }

    pub fn cells(&self) -> &[CellId] {
        &self.cells
    }

    pub fn iter(&self) -> impl Iterator<Item = CellId> + '_ {
        self.cells().iter().copied()
    }
}

/// Entities that client is aware of.
#[derive(Clone, Debug, Default)]
pub struct KnownEntities {
    pub entities: HashMap<EntityId, Entity>,
    pub inventories: HashMap<EntityId, Inventory>,
}

impl KnownEntities {
    pub fn new() -> Self {
        Self {
            entities: HashMap::default(),
            inventories: HashMap::default(),
        }
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    pub fn remove(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HostState {
    pub entity: Option<EntityId>,
}
