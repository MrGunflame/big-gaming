use std::borrow::Borrow;
use std::fmt::{self, Debug, Formatter};

use ahash::AHashMap;
use bevy_ecs::component::Component;

/// An object that can be interacted with.
#[derive(Debug, Component)]
pub struct Interactable {
    /// An optional displayed name of the object.
    pub name: Option<String>,
    pub interactions: Interactions,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionId();

#[derive(Debug)]
pub struct Interactions {
    interactions: AHashMap<InteractionId, Interaction>,
}

impl Interactions {
    pub fn new() -> Self {
        Self {
            interactions: AHashMap::new(),
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

    pub fn iter(&self) -> impl Iterator<Item = &Interaction> {
        self.interactions.values()
    }
}

pub struct Interaction {
    pub id: InteractionId,
    pub title: String,
    pub executor: Box<dyn FnMut() + Send + Sync + 'static>,
}

impl Debug for Interaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interaction")
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}
