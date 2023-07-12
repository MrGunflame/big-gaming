//! Scripts assigned to an entity.

use std::collections::HashMap;

use bevy_ecs::system::Resource;
use game_common::entity::EntityId;
use game_common::events::EventKind;

use crate::Handle;

/// Store for what events have which assigned scripts.
#[derive(Clone, Debug, Resource)]
pub struct Scripts {
    scripts: HashMap<(Option<EntityId>, EventKind), Vec<Handle>>,
}

impl Scripts {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
        }
    }

    pub fn push(&mut self, entity: Option<EntityId>, event: EventKind, handle: Handle) {
        self.scripts
            .entry((entity, event))
            .or_default()
            .push(handle);
    }

    pub fn get(&self, entity: Option<EntityId>, event: EventKind) -> Option<&[Handle]> {
        self.scripts.get(&(entity, event)).map(|s| s.as_slice())
    }

    pub fn remove(&mut self, entity: Option<EntityId>, event: EventKind) {
        self.scripts.remove(&(entity, event));
    }
}
