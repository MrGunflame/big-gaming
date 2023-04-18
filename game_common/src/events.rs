//! Event pipeline

use std::collections::VecDeque;

use bevy_ecs::system::Resource;

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

    pub fn push(&mut self, event: EntityEvent) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<EntityEvent> {
        self.events.pop_front()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Action { entity: EntityId, invoker: EntityId },
    Collision { entity: EntityId, other: EntityId },
}

impl Event {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Action {
                entity: _,
                invoker: _,
            } => EventKind::Action,
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
