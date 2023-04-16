//! Scripts assigned to an entity.

use std::collections::HashMap;

use bevy_ecs::system::Resource;
use game_common::entity::EntityId;

use crate::events::Event;
use crate::Handle;

#[derive(Clone, Debug, Resource)]
pub struct Scripts {
    scripts: HashMap<(EntityId, Event), Vec<Handle>>,
}

impl Scripts {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
        }
    }

    pub fn push(&mut self, entity: EntityId, event: Event, handle: Handle) {
        self.scripts
            .entry((entity, event))
            .or_default()
            .push(handle);
    }

    pub fn get(&mut self, entity: EntityId, event: Event) -> Option<&[Handle]> {
        self.scripts.get(&(entity, event)).map(|s| s.as_slice())
    }
}
