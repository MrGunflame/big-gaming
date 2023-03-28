use bevy_ecs::system::Resource;
use game_common::components::combat::Health;
use game_common::entity::{Entity, EntityData, EntityId};
use game_common::world::terrain::Heightmap;
use game_common::world::CellId;
use glam::{Quat, Vec3};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::sync::Arc;
use std::time::Instant;

use crate::conn::ConnectionId;
use crate::proto::sequence::Sequence;
use crate::world::Override;

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

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    // FIXME: Can be hashset
    entities: HashMap<EntityId, Entity>,
}

impl Snapshot {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn update(&mut self, entity: Entity) {
        match self.entities.get_mut(&entity.id) {
            Some(ent) => *ent = entity,
            None => {
                self.entities.insert(entity.id, entity);
            }
        }
    }

    pub fn delta(&self, new: &Self) -> Vec<EntityChange> {
        let mut entities = new.entities.clone();

        let mut delta = Vec::new();

        for (id, body) in &self.entities {
            match entities.remove(id) {
                Some(new) => {
                    if body.transform.translation != new.transform.translation {
                        delta.push(EntityChange::Translate {
                            id: *id,
                            translation: new.transform.translation,
                            cell: Some(TransferCell {
                                from: body.transform.translation.into(),
                                to: new.transform.translation.into(),
                            }),
                        });
                    }

                    if body.transform.rotation != new.transform.rotation {
                        delta.push(EntityChange::Rotate {
                            id: *id,
                            rotation: new.transform.rotation,
                        });
                    }

                    // if body != &new {
                    //     delta.push(EntityChange::Update { id: *id, data: new });
                    // }
                }
                None => {
                    delta.push(EntityChange::Destroy { id: *id });
                }
            }
        }

        // New entities
        for (id, body) in entities {
            delta.push(EntityChange::Create { id, data: body });
        }

        delta
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionMessage {
    pub id: ConnectionId,
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
        data: EntityData,
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
    WorldTerrain {
        cell: CellId,
        height: Heightmap,
    },
}

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
            Self::WorldTerrain { cell: _, height: _ } => None,
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

#[derive(Clone, Debug)]
pub enum EntityChange {
    Create {
        id: EntityId,
        data: Entity,
    },
    Translate {
        id: EntityId,
        translation: Vec3,
        cell: Option<TransferCell>,
    },
    Rotate {
        id: EntityId,
        rotation: Quat,
    },
    Health {
        id: EntityId,
        health: Health,
    },
    // Update { id: EntityId, data: Entity },
    Destroy {
        id: EntityId,
    },
    CreateHost {
        id: EntityId,
    },
    DestroyHost {
        id: EntityId,
    },
    CreateTerrain {
        cell: CellId,
        height: Heightmap,
    },
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
