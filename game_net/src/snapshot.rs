use bevy_ecs::system::Resource;
use game_common::components::actions::ActionId;
use game_common::components::combat::Health;
use game_common::components::inventory::InventoryId;
use game_common::components::items::ItemId;
use game_common::entity::EntityId;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::EntityBody;
use game_common::world::CellId;
use glam::{Quat, Vec3};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::sync::Arc;

use crate::conn::ConnectionId;

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
    Connected,
    Disconnected,
    EntityCreate {
        id: EntityId,
        translation: Vec3,
        rotation: Quat,
        data: EntityBody,
    },
    EntityDestroy {
        id: EntityId,
    },
    EntityTranslate {
        id: EntityId,
        translation: Vec3,
    },
    EntityRotate {
        id: EntityId,
        rotation: Quat,
    },
    EntityVelocity {
        id: EntityId,
        linvel: Vec3,
        angvel: Vec3,
    },
    EntityHealth {
        id: EntityId,
        health: Health,
    },
    EntityAction {
        id: EntityId,
        action: ActionId,
    },
    SpawnHost {
        id: EntityId,
    },
    ReceivedCommands {
        ids: Vec<Response>,
    },
    InventoryItemAdd {
        entity: EntityId,
        id: InventoryId,
        item: ItemId,
    },
    InventoryItemRemove {
        entity: EntityId,
        id: InventoryId,
    },
    InventoryUpdate {
        entity: EntityId,
        id: InventoryId,
        equipped: Option<bool>,
        hidden: Option<bool>,
    },
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandId(pub u32);

impl Command {
    pub const fn id(&self) -> Option<EntityId> {
        match self {
            Self::Connected => None,
            Self::Disconnected => None,
            Self::EntityCreate {
                id,
                translation: _,
                rotation: _,
                data: _,
            } => Some(*id),
            Self::EntityDestroy { id } => Some(*id),
            Self::EntityTranslate { id, translation: _ } => Some(*id),
            Self::EntityRotate { id, rotation: _ } => Some(*id),
            Self::EntityVelocity {
                id,
                linvel: _,
                angvel: _,
            } => Some(*id),
            Self::EntityHealth { id, health: _ } => Some(*id),
            Self::EntityAction { id, action: _ } => Some(*id),
            Self::SpawnHost { id } => Some(*id),
            Self::InventoryItemAdd {
                entity,
                id: _,
                item: _,
            } => Some(*entity),
            Self::InventoryItemRemove { entity, id: _ } => Some(*entity),
            Self::InventoryUpdate {
                entity,
                id: _,
                equipped: _,
                hidden: _,
            } => Some(*entity),
            Self::ReceivedCommands { ids: _ } => None,
        }
    }
}

#[derive(Clone, Debug, Default, Resource)]
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
