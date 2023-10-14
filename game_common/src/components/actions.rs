#![deprecated]

use std::fmt::Debug;

use bytemuck::{Pod, Zeroable};

use crate::record::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct ActionId(pub RecordReference);

#[derive(Clone, Debug, Default)]
pub struct Actions {
    actions: Vec<ActionId>,
}

impl Actions {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn push(&mut self, action: ActionId) {
        self.actions.push(action);
    }

    pub fn iter(&self) -> impl Iterator<Item = ActionId> + '_ {
        self.actions.iter().copied()
    }
}
