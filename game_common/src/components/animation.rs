use std::collections::VecDeque;

use crate::id::WeakId;

use super::transform::Transform;

#[derive(Clone, Debug, Default)]
pub struct AnimationQueue {
    queue: VecDeque<AnimationId>,
}

impl AnimationQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, animation: AnimationId) {
        self.queue.push_back(animation);
    }

    pub fn pop(&mut self) -> Option<AnimationId> {
        self.queue.pop_front()
    }
}

/// A unique identifier for an animation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnimationId(pub WeakId<u32>);

impl AnimationId {
    pub const DEATH: Self = Self(WeakId(1));
}

#[derive(Clone, Debug)]
pub struct Animation {}
