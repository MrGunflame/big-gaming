use std::collections::VecDeque;

use game_common::entity::EntityId;

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
    SpawnHost(EntityId),
}
