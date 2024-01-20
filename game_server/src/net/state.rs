use ahash::HashMap;
use game_common::components::components::{Components, RawComponent};
use game_common::components::inventory::Inventory;
use game_common::components::PlayerId;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::cell::square;
use game_common::world::control_frame::ControlFrame;
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
    pub components: HashMap<EntityId, Components>,
    pub inventories: HashMap<EntityId, Inventory>,
}

impl KnownEntities {
    pub fn new() -> Self {
        Self {
            components: HashMap::default(),
            inventories: HashMap::default(),
        }
    }

    pub fn insert(
        &mut self,
        entity: EntityId,
        component_id: RecordReference,
        component: RawComponent,
    ) {
        self.components
            .entry(entity)
            .or_default()
            .insert(component_id, component);
    }

    pub fn remove(&mut self, entity: EntityId, component_id: RecordReference) {
        self.components
            .remove(&entity)
            .unwrap()
            .remove(component_id);
    }

    pub fn despawn(&mut self, entity: EntityId) {
        self.components.remove(&entity);
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.components.contains_key(&id)
    }

    pub fn clear(&mut self) {
        self.components.clear();
        self.inventories.clear();
    }

    pub fn get(&self, entity: EntityId, component_id: RecordReference) -> Option<&RawComponent> {
        let components = self.components.get(&entity)?;
        components.get(component_id)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HostState {
    pub entity: Option<EntityId>,
    pub player: Option<PlayerId>,
}
