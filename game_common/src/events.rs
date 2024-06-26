//! Event pipeline

use std::collections::VecDeque;

use game_wasm::encoding::{Encode, Primitive, Writer};
use game_wasm::player::PlayerId;

use crate::components::actions::ActionId;
use crate::entity::EntityId;
use crate::world::CellId;

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
    PlayerConnect(PlayerConnect),
    PlayerDisconnect(PlayerDisconnect),
    CellLoad(CellLoad),
    CellUnload(CellUnload),
}

impl Event {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Action(_) => EventKind::Action,
            Self::Collision(_) => EventKind::Collision,
            Self::PlayerConnect(_) => EventKind::PlayerConnect,
            Self::PlayerDisconnect(_) => EventKind::PlayerDisconnect,
            Self::CellLoad(_) => EventKind::CellLoad,
            Self::CellUnload(_) => EventKind::CellUnload,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Action,
    Collision,
    PlayerConnect,
    PlayerDisconnect,
    CellLoad,
    CellUnload,
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

#[derive(Copy, Clone, Debug)]
pub struct PlayerConnect {
    pub player: PlayerId,
}

impl Encode for PlayerConnect {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        writer.write(Primitive::PlayerId, &self.player.to_bits().to_le_bytes());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PlayerDisconnect {
    pub player: PlayerId,
}

impl Encode for PlayerDisconnect {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        writer.write(Primitive::PlayerId, &self.player.to_bits().to_le_bytes());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CellLoad {
    pub id: CellId,
}

impl Encode for CellLoad {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        let (x, y, z) = self.id.as_parts();
        writer.write(Primitive::Bytes, &x.to_le_bytes());
        writer.write(Primitive::Bytes, &y.to_le_bytes());
        writer.write(Primitive::Bytes, &z.to_le_bytes());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CellUnload {
    pub id: CellId,
}

impl Encode for CellUnload {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        let (x, y, z) = self.id.as_parts();
        writer.write(Primitive::Bytes, &x.to_le_bytes());
        writer.write(Primitive::Bytes, &y.to_le_bytes());
        writer.write(Primitive::Bytes, &z.to_le_bytes());
    }
}
