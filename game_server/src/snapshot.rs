use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use bevy::prelude::{Entity, Resource};
use game_common::net::ServerEntity;
use game_net::proto::Frame;
use parking_lot::Mutex;

#[derive(Clone, Debug, Default, Resource)]
pub struct CommandQueue {
    queue: Arc<Mutex<VecDeque<Frame>>>,
}

impl CommandQueue {
    pub fn push(&self, frame: Frame) {
        let mut queue = self.queue.lock();
        queue.push_back(frame);
    }

    pub fn pop(&self) -> Option<Frame> {
        let mut queue = self.queue.lock();
        queue.pop_front()
    }
}
