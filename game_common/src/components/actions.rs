use std::fmt::Debug;

use bytemuck::{Pod, Zeroable};

use crate::entity::EntityId;
use crate::record::RecordReference;

use super::inventory::InventoryId;

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
}

#[derive(Clone, Debug)]
pub struct Action {
    pub entity: EntityId,
    pub id: ActionId,
    pub item: Option<InventoryId>,
}
