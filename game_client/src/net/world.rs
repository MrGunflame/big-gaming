use std::collections::VecDeque;

use game_common::components::inventory::InventorySlotId;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::entity::Entity;
use glam::{Quat, Vec3};

#[derive(Clone, Debug, Default)]
pub struct CommandBuffer {
    buffer: VecDeque<Command>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn push(&mut self, cmd: Command) {
        self.buffer.push_back(cmd);
    }

    pub fn pop(&mut self) -> Option<Command> {
        self.buffer.pop_front()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Spawn(Entity),
    Despawn(EntityId),
    Translate {
        entity: EntityId,
        dst: Vec3,
    },
    Rotate {
        entity: EntityId,
        dst: Quat,
    },
    SpawnHost(EntityId),
    ComponentAdd {
        entity: EntityId,
        component: RecordReference,
    },
    ComponentRemove {
        entity: EntityId,
        component: RecordReference,
    },
    InventoryItemEquip {
        entity: EntityId,
        slot: InventorySlotId,
    },
    InventoryItemUnequip {
        entity: EntityId,
        slot: InventorySlotId,
    },
}
