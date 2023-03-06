use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityId};
use glam::{Quat, Vec3};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::sync::Arc;
use std::time::Instant;

use crate::conn::ConnectionId;
use crate::proto::EntityKind;

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
    pub snapshot: SnapshotId,
    pub command: Command,
}

#[derive(Clone, Debug)]
pub enum Command {
    Connected,
    Disconnected,
    EntityCreate {
        id: EntityId,
        kind: EntityKind,
        translation: Vec3,
        rotation: Quat,
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
    SpawnHost {
        id: EntityId,
    },
}

impl Command {
    pub const fn id(&self) -> Option<EntityId> {
        match self {
            Self::Connected => None,
            Self::Disconnected => None,
            Self::EntityCreate {
                id,
                kind: _,
                translation: _,
                rotation: _,
            } => Some(*id),
            Self::EntityDestroy { id } => Some(*id),
            Self::EntityTranslate { id, translation: _ } => Some(*id),
            Self::EntityRotate { id, rotation: _ } => Some(*id),
            Self::EntityVelocity {
                id,
                linvel: _,
                angvel: _,
            } => Some(*id),
            Self::SpawnHost { id } => Some(*id),
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
    Create { id: EntityId, data: Entity },
    Translate { id: EntityId, translation: Vec3 },
    Rotate { id: EntityId, rotation: Quat },
    // Update { id: EntityId, data: Entity },
    Destroy { id: EntityId },
    CreateHost { id: EntityId },
    DestroyHost { id: EntityId },
}

pub struct Patch {}

#[derive(Clone, Debug, Resource)]
pub struct Snapshots {
    snapshots: VecDeque<(Instant, SnapshotId)>,
    next_id: SnapshotId,
}

impl Snapshots {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::new(),
            next_id: SnapshotId(0),
        }
    }

    pub fn push(&mut self) {
        self.push_in(Instant::now());
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Returns the id of the first snapshot that happened at or after
    /// `ts`.
    pub fn get(&self, ts: Instant) -> Option<SnapshotId> {
        let mut index = 0;
        while index < self.snapshots.len() {
            let (t, id) = self.snapshots[index];

            if ts <= t {
                return Some(id);
            }

            index += 1;
        }

        None
    }

    pub fn remove(&mut self, id: SnapshotId) {
        self.snapshots.retain(|(_, i)| *i != id);
    }

    pub fn newest(&self) -> Option<SnapshotId> {
        self.snapshots.back().map(|(_, x)| *x)
    }

    pub fn oldest(&self) -> Option<SnapshotId> {
        self.snapshots.front().map(|(_, x)| *x)
    }

    fn push_in(&mut self, instant: Instant) {
        let id = self.next_id;
        self.next_id += 1;

        self.snapshots.push_back((instant, id));
    }
}

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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{SnapshotId, Snapshots};

    #[test]
    fn test_snapshots() {
        let mut snapshots = Snapshots::new();

        let now = Instant::now();

        let t1 = now;
        let t2 = now + Duration::new(1, 0);
        let t3 = now + Duration::new(2, 0);

        snapshots.push_in(t1);
        snapshots.push_in(t2);
        snapshots.push_in(t3);

        assert_eq!(snapshots.get(t1), Some(SnapshotId(0)));
        assert_eq!(snapshots.get(t2), Some(SnapshotId(1)));
        assert_eq!(snapshots.get(t3), Some(SnapshotId(2)));

        assert_eq!(snapshots.get(t1 + Duration::new(0, 1)), Some(SnapshotId(1)));
    }
}
