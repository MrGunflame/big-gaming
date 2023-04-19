use std::collections::VecDeque;
use std::fmt::Debug;

use bevy_ecs::system::Resource;

use crate::entity::EntityId;

use super::components::RecordReference;

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

#[derive(Clone, Debug, Default, Resource)]
pub struct ActionQueue {
    queue: VecDeque<Action>,
}

impl ActionQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, action: Action) {
        self.queue.push_back(action);
    }

    pub fn pop(&mut self) -> Option<Action> {
        self.queue.pop_front()
    }
}

#[derive(Clone, Debug)]
pub struct Action {
    pub entity: EntityId,
    pub id: ActionId,
}
