use std::borrow::Borrow;
use std::collections::HashSet;

use crate::id::WeakId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A unique identifier for a faction.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct FactionId(pub WeakId<u32>);

/// The diplomatic standings between two factions.
///
/// The default `FactionStanding` is `Neutral`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum FactionStanding {
    /// The factions are enemied with each other and will attack on sight.
    Enemied,
    /// The factions will not attack each other, until either one threatens the other.
    ///
    /// This is the default `FactionStanding`.
    #[default]
    Neutral,
    /// The factions are allied with each other and will not attack each other.
    Allied,
}

/// A list of factions a actor is part of.
#[derive(Clone, Debug, Default)]
pub struct ActorFactions {
    factions: HashSet<FactionId>,
}

impl ActorFactions {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains<T>(&self, faction: T) -> bool
    where
        T: Borrow<FactionId>,
    {
        self.factions.contains(faction.borrow())
    }

    pub fn insert(&mut self, faction: FactionId) {
        self.factions.insert(faction);
    }

    pub fn remove<T>(&mut self, id: T) -> bool
    where
        T: Borrow<FactionId>,
    {
        self.factions.remove(id.borrow())
    }
}
