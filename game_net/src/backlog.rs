//! Entity backlogs
//!
//! When packets get lost that create entities the peer might sent updates for entities that
//! don't exist locally. They should be buffered until the retransmission arrives.

use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::world::snapshot::EntityChange;

#[derive(Clone, Debug, Default)]
pub struct Backlog {
    entities: HashMap<EntityId, Vec<EntityChange>>,
}

impl Backlog {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn push(&mut self, id: EntityId, event: EntityChange) {
        self.entities.entry(id).or_default().push(event);
    }

    pub fn remove(&mut self, id: EntityId) -> Option<Vec<EntityChange>> {
        self.entities.remove(&id)
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }
}
