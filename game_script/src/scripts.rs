//! Scripts assigned to an entity.

use std::collections::HashMap;

use crate::events::Event;
use crate::Handle;

#[derive(Clone, Debug)]
pub struct Scripts {
    scripts: HashMap<Event, Vec<Handle>>,
}

impl Scripts {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
        }
    }

    pub fn push(&mut self, event: Event, handle: Handle) {
        self.scripts.entry(event).or_default().push(handle);
    }

    pub fn get(&mut self, event: Event) -> Option<&[Handle]> {
        self.scripts.get(&event).map(|s| s.as_slice())
    }
}
