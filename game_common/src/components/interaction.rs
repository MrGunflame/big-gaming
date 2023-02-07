//! Interactions of an entitiy

use std::borrow::Borrow;
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::iter::FusedIterator;
use std::sync::Arc;

use ahash::RandomState;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use indexmap::IndexMap;

use crate::id::WeakId;

/// `fn(target: Entity, actor: Entity, world: &mut World)`
pub type ArcExecutor = Arc<dyn Fn(Entity, Entity, &mut World) + Send + Sync + 'static>;

#[derive(Resource)]
pub struct InteractionQueue {
    // FIXME: This should better be a `Vec` that is fully iterated and then dropped.
    // `(ArcExec, target: Entity, actor: Entity)`
    queue: VecDeque<(ArcExecutor, Entity, Entity)>,
}

impl InteractionQueue {
    pub fn new() -> Self {
        // Interaction happen very often, we start with a preallocated
        // buffer to prevent tiny reallocation.
        Self::with_capacity(32)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, interaction: &Interaction, target: Entity, actor: Entity) {
        self.queue
            .push_back((interaction.executor.clone(), target, actor));
    }

    pub fn run(&mut self, world: &mut World) {
        while let Some((exec, target, actor)) = self.queue.pop_front() {
            exec(target, actor, world);
        }
    }
}

impl Default for InteractionQueue {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InteractionId(pub WeakId<u32>);

#[derive(Debug, Component)]
pub struct Interactions {
    interactions: IndexMap<InteractionId, Interaction, RandomState>,
}

impl Interactions {
    pub fn new() -> Self {
        Self {
            interactions: IndexMap::with_hasher(RandomState::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            interactions: IndexMap::with_capacity_and_hasher(capacity, RandomState::new()),
        }
    }

    pub fn insert(&mut self, interaction: Interaction) {
        self.interactions.insert(interaction.id, interaction);
    }

    pub fn get<T>(&self, id: T) -> Option<&Interaction>
    where
        T: Borrow<InteractionId>,
    {
        self.interactions.get(id.borrow())
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.interactions.values(),
        }
    }
}

pub struct Interaction {
    pub id: InteractionId,
    /// The text displayed for the interaction.
    pub text: Option<String>,
    pub executor: ArcExecutor,
}

impl From<Interaction> for Interactions {
    fn from(value: Interaction) -> Self {
        let mut interactions = Interactions::with_capacity(1);
        interactions.insert(value);
        interactions
    }
}

impl Debug for Interaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interaction")
            .field("text", &self.text)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    iter: indexmap::map::Values<'a, InteractionId, Interaction>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Interaction;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}
