//! Event pipeline

use std::collections::VecDeque;

use bevy_ecs::system::Resource;

use crate::components::actions::ActionId;
use crate::entity::EntityId;

#[derive(Clone, Debug, Default, Resource)]
pub struct EventQueue {
    events: VecDeque<EntityEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn push(&mut self, event: EntityEvent) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<EntityEvent> {
        self.events.pop_front()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Action(ActionEvent),
    Collision { entity: EntityId, other: EntityId },
}

impl Event {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Action(_) => EventKind::Action,
            Self::Collision {
                entity: _,
                other: _,
            } => EventKind::Collision,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Action,
    Collision,
}

#[derive(Clone, Debug)]
pub struct EntityEvent {
    pub entity: EntityId,
    pub event: Event,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionEvent {
    pub entity: EntityId,
    pub invoker: EntityId,
    pub action: ActionId,
}