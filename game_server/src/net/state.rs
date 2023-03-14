use game_common::entity::EntityId;
use game_common::world::CellId;

#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub full_update: bool,
    /// Cells loaded by the peer.
    pub cells: Vec<CellId>,
    /// The entity that is the host.
    pub id: Option<EntityId>,
}
