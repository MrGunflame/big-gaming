use game_common::components::PlayerId;
use game_common::entity::EntityId;
use game_common::world::cell::CubeIter;
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
        let cells = CubeIter::new(origin, 0).collect();

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

        self.cells.clear();
        self.cells.extend(CubeIter::new(origin, distance));
    }

    pub fn cells(&self) -> &[CellId] {
        &self.cells
    }

    pub fn iter(&self) -> impl Iterator<Item = CellId> + '_ {
        self.cells().iter().copied()
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HostState {
    pub entity: Option<EntityId>,
    pub player: Option<PlayerId>,
}
