use game_common::entity::EntityId;
use game_common::world::CellId;

#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub full_update: bool,
    /// Cells loaded by the peer.
    pub cells: Vec<CellId>,
    /// The entity that is the host.
    pub id: Option<EntityId>,
    /// The snapshot index that the client's view is located at (currently modified).
    ///
    /// `head - 1..head` is the delta period.
    pub head: usize,
}
