use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityId};
use glam::{Quat, Vec3};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::conn::ConnectionId;
use crate::proto::EntityKind;

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
}

pub struct Patch {}
