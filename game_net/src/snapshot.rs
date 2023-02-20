use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use game_common::world::entity::Entity as EntityBody;
use glam::{Quat, Vec3};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::conn::ConnectionId;
use crate::entity::Entities;
use crate::proto::Frame;

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    entities: HashMap<Entity, EntityBody>,
}

impl Snapshot {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn update(&mut self, id: Entity, new: EntityBody) {
        match self.entities.get_mut(&id) {
            Some(ent) => *ent = new,
            None => {
                self.entities.insert(id, new);
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
                        delta.push(EntityChange::Update {
                            id: *id,
                            content: new,
                        });
                    }
                }
                None => {
                    delta.push(EntityChange::Destroy(*id));
                }
            }
        }

        // New entities
        for (id, body) in entities {
            delta.push(EntityChange::Create { id, content: body });
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
        id: Entity,
        translation: Vec3,
        rotation: Quat,
    },
    EntityDestroy {
        id: Entity,
    },
    EntityTranslate {
        id: Entity,
        translation: Vec3,
    },
    EntityRotate {
        id: Entity,
        rotation: Quat,
    },
    PlayerJoin,
    PlayerLeave,
    SpawnHost {
        id: Entity,
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
    Create { id: Entity, content: EntityBody },
    Update { id: Entity, content: EntityBody },
    Destroy(Entity),
}
