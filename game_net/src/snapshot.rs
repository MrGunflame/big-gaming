use bevy_ecs::system::Resource;
use game_common::entity::{Entity, EntityId};
use game_common::net::ServerEntity;
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
                    if body != &new {
                        delta.push(EntityChange::Update { id: *id, data: new });
                    }
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
    PlayerJoin,
    PlayerLeave,
    SpawnHost {
        id: EntityId,
    },
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
    Update { id: EntityId, data: Entity },
    Destroy { id: EntityId },
}
