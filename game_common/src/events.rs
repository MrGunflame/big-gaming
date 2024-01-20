//! Event pipeline

use std::collections::VecDeque;

use game_wasm::player::PlayerId;

use crate::components::actions::ActionId;
use crate::entity::EntityId;

#[derive(Clone, Debug, Default)]
pub struct EventQueue {
    events: VecDeque<Event>,
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, event: Event) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.events.pop_front()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Action(ActionEvent),
    Collision(CollisionEvent),
    PlayerConnect(PlayerId),
    PlayerDisconnect(PlayerId),
}

impl Event {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Action(_) => EventKind::Action,
            Self::Collision(_) => EventKind::Collision,
            Self::PlayerConnect(_) => EventKind::PlayerConnect,
            Self::PlayerDisconnect(_) => EventKind::PlayerDisconnect,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Action,
    Collision,
    PlayerConnect,
    PlayerDisconnect,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionEvent {
    pub entity: EntityId,
    pub invoker: EntityId,
    pub action: ActionId,
    pub data: Vec<u8>,
}

impl From<ActionEvent> for Event {
    #[inline]
    fn from(event: ActionEvent) -> Self {
        Self::Action(event)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CollisionEvent {
    pub entity: EntityId,
    pub other: EntityId,
}

impl From<CollisionEvent> for Event {
    #[inline]
    fn from(event: CollisionEvent) -> Self {
        Self::Collision(event)
    }
}
