use game_common::components::actions::ActionId;
use game_common::components::combat::Health;
use game_common::components::inventory::InventoryId;
use game_common::components::items::ItemId;
use game_common::net::ServerEntity;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::EntityBody;
use game_common::world::CellId;
use glam::{Quat, Vec3};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::conn::ConnectionId;
use crate::proto::MoveBits;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Response {
    pub id: CommandId,
    pub status: Status,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// The command was acknowledged by the remote peer.
    Received,
    /// The command was dropped because it is no longer relevant. Another command takes the place
    /// of this command.
    Overwritten,
    Dropped,
}

#[derive(Clone, Debug)]
pub struct ConnectionMessage {
    pub id: Option<CommandId>,
    pub conn: ConnectionId,
    pub control_frame: ControlFrame,
    pub command: Command,
}

#[derive(Clone, Debug)]
pub enum Command {
    Connected(Connected),
    Disconnected,
    EntityCreate(EntityCreate),
    EntityDestroy(EntityDestroy),
    EntityTranslate(EntityTranslate),
    EntityRotate(EntityRotate),
    EntityHealth(EntityHealth),
    EntityAction(EntityAction),
    SpawnHost(SpawnHost),
    ReceivedCommands(Vec<Response>),
    InventoryItemAdd(InventoryItemAdd),
    InventoryItemRemove(InventoryItemRemove),
    InventoryUpdate(InventoryUpdate),
    PlayerMove(PlayerMove),
}

#[derive(Copy, Clone, Debug)]
pub struct Connected {
    /// The negotiated interpolation/buffer delay of the peer.
    pub peer_delay: ControlFrame,
}

#[derive(Clone, Debug)]
pub struct EntityCreate {
    pub id: ServerEntity,
    pub translation: Vec3,
    pub rotation: Quat,
    pub data: EntityBody,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityDestroy {
    pub id: ServerEntity,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityTranslate {
    pub id: ServerEntity,
    pub translation: Vec3,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityRotate {
    pub id: ServerEntity,
    pub rotation: Quat,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityHealth {
    pub id: ServerEntity,
    pub health: Health,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityAction {
    pub id: ServerEntity,
    pub action: ActionId,
}

#[derive(Copy, Clone, Debug)]
pub struct SpawnHost {
    pub id: ServerEntity,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryItemAdd {
    pub entity: ServerEntity,
    pub slot: InventoryId,
    pub item: ItemId,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryItemRemove {
    pub entity: ServerEntity,
    pub slot: InventoryId,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryUpdate {
    pub entity: ServerEntity,
    pub slot: InventoryId,
    pub equipped: Option<bool>,
    pub hidden: Option<bool>,
}

#[derive(Copy, Clone, Debug)]
pub struct PlayerMove {
    pub entity: ServerEntity,
    pub bits: MoveBits,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandId(pub u32);

impl Command {
    pub const fn id(&self) -> Option<ServerEntity> {
        match self {
            Self::Connected(_) => None,
            Self::Disconnected => None,
            Self::EntityCreate(cmd) => Some(cmd.id),
            Self::EntityDestroy(cmd) => Some(cmd.id),
            Self::EntityTranslate(cmd) => Some(cmd.id),
            Self::EntityRotate(cmd) => Some(cmd.id),
            Self::EntityHealth(cmd) => Some(cmd.id),
            Self::EntityAction(cmd) => Some(cmd.id),
            Self::SpawnHost(cmd) => Some(cmd.id),
            Self::InventoryItemAdd(cmd) => Some(cmd.entity),
            Self::InventoryItemRemove(cmd) => Some(cmd.entity),
            Self::InventoryUpdate(cmd) => Some(cmd.entity),
            Self::ReceivedCommands(_) => None,
            Self::PlayerMove(cmd) => Some(cmd.entity),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandQueue {
    queue: Arc<Mutex<VecDeque<ConnectionMessage>>>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::default(),
        }
    }

    pub fn push(&self, msg: ConnectionMessage) {
        let mut queue = self.queue.lock();
        queue.push_back(msg);
    }

    pub fn pop(&self) -> Option<ConnectionMessage> {
        let mut queue = self.queue.lock();
        queue.pop_front()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TransferCell {
    pub from: CellId,
    pub to: CellId,
}

impl TransferCell {
    #[inline]
    pub fn new<T, U>(from: T, to: U) -> Option<Self>
    where
        T: Into<CellId>,
        U: Into<CellId>,
    {
        let from = from.into();
        let to = to.into();

        if from == to {
            None
        } else {
            Some(Self { from, to })
        }
    }
}

pub struct Patch {}
