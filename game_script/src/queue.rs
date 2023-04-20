use std::collections::VecDeque;

use game_common::entity::EntityId;

#[derive(Clone, Debug, Default)]
pub struct CommandQueue {
    commands: VecDeque<Command>,
}

impl CommandQueue {
    pub(crate) fn new() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }

    pub(crate) fn push(&mut self, cmd: Command) {
        self.commands.push_back(cmd);
    }

    pub(crate) fn pop(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Command {
    DestroyEntity(EntityId),
}
