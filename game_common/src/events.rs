//! Event pipeline

use std::collections::VecDeque;

use bevy_ecs::system::Resource;

use crate::components::actions::ActionId;
use crate::components::inventory::InventoryId;
use crate::entity::EntityId;
use crate::world::CellId;

#[derive(Clone, Debug, Default, Resource)]
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
    Equip(EquipEvent),
    Unequip(UnequipEvent),
    CellLoad(CellLoadEvent),
    CellUnload(CellUnloadEvent),
}

impl Event {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Action(_) => EventKind::Action,
            Self::Collision(_) => EventKind::Collision,
            Self::Equip(_) => EventKind::Equip,
            Self::Unequip(_) => EventKind::Unequip,
            Self::CellLoad(_) => EventKind::CellLoad,
            Self::CellUnload(_) => EventKind::CellUnload,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Action,
    Collision,
    Equip,
    Unequip,
    CellLoad,
    CellUnload,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionEvent {
    pub entity: EntityId,
    pub invoker: EntityId,
    pub action: ActionId,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellLoadEvent {
    pub cell: CellId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellUnloadEvent {
    pub cell: CellId,
}
