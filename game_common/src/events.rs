//! Event pipeline

use std::collections::VecDeque;

use bevy_ecs::system::Resource;

use crate::components::actions::ActionId;
use crate::components::inventory::InventoryId;
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
    Collision(CollisionEvent),
    Equip(EquipEvent),
    Unequip(UnequipEvent),
}

impl Event {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Action(_) => EventKind::Action,
            Self::Collision(_) => EventKind::Collision,
            Self::Equip(_) => EventKind::Equip,
            Self::Unequip(_) => EventKind::Unequip,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Action,
    Collision,
    Equip,
    Unequip,
}

#[derive(Clone, Debug)]
pub struct EntityEvent {
    // FIXME: Why do Events already have an entity field?
    // This should be merged.
    pub entity: EntityId,
    pub event: Event,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionEvent {
    pub entity: EntityId,
    pub invoker: EntityId,
    pub action: ActionId,
}

#[derive(Copy, Clone, Debug)]
pub struct CollisionEvent {
    pub entity: EntityId,
    pub other: EntityId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EquipEvent {
    pub entity: EntityId,
    pub item: InventoryId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnequipEvent {
    pub entity: EntityId,
    pub item: InventoryId,
}
