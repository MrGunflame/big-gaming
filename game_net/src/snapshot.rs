use bevy_ecs::system::Resource;
use game_common::components::combat::Health;
use game_common::entity::EntityId;
use game_common::world::entity::EntityBody;
use game_common::world::snapshot::EntityChange;
use game_common::world::CellId;
use glam::{Quat, Vec3};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::sync::Arc;
use std::time::Instant;

use crate::conn::ConnectionId;

/// A temporary identifier for a snapshot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SnapshotId(pub u32);

impl Add<u32> for SnapshotId {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl AddAssign<u32> for SnapshotId {
    #[inline]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

impl Sub<u32> for SnapshotId {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl SubAssign<u32> for SnapshotId {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionMessage {
    pub id: Option<CommandId>,
    pub conn: ConnectionId,
    pub snapshot: Instant,
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
    SpawnHost {
        id: EntityId,
    },
    ReceivedCommands {
        ids: Vec<CommandId>,
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
            Self::SpawnHost { id } => Some(*id),
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

#[derive(Clone, Debug, Default, Resource)]
pub struct DeltaQueue {
    queue: VecDeque<EntityChange>,
}

impl DeltaQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, change: EntityChange) {
        self.queue.push_back(change);
    }

    pub fn peek(&mut self) -> Option<&EntityChange> {
        self.queue.front()
    }

    pub fn pop(&mut self) -> Option<EntityChange> {
        self.queue.pop_front()
    }
}
