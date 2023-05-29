use std::collections::VecDeque;

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use crate::id::WeakId;

use super::transform::Transform;

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

impl AnimationId {
    pub const DEATH: Self = Self(WeakId(1));
}

#[derive(Clone, Debug)]
pub struct Animation {}

/// A animation skeleton, consisting of multiple connected [`Bone`]s.
#[derive(Copy, Clone, Debug, Component)]
pub struct Skeleton {
    /// The entity that is the root bone of the skeleton.
    ///
    /// The root entity must have [`Bone`] component.
    pub root: Entity,
}

#[derive(Clone, Debug, Default, Component)]
pub struct Bone {
    /// A list of other bones that are attached to the end of this bone.
    pub children: Box<[Entity]>,
    /// Offset from the parent.
    pub offset: Transform,
}
