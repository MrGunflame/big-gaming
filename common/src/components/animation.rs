use std::collections::VecDeque;

use bevy_ecs::component::Component;

use crate::id::WeakId;

#[derive(Clone, Debug, Default, Component)]
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

#[derive(Clone, Debug)]
pub struct Animation {}
