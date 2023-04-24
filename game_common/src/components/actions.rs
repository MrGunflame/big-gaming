use std::collections::VecDeque;
use std::fmt::Debug;

use bevy_ecs::system::Resource;

use crate::entity::EntityId;

use super::components::RecordReference;
use super::inventory::InventoryId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionId(pub RecordReference);

#[derive(Clone, Debug, Default)]
pub struct Actions {
    actions: Vec<Action>,
}

impl Actions {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn push(&mut self, action: Action) {
        self.actions.push(action);
    }
}

#[derive(Clone, Debug)]
pub struct Action {
    pub entity: EntityId,
    pub id: ActionId,
    pub item: Option<InventoryId>,
}
