use std::collections::HashMap;
use std::num::NonZeroU8;

use ahash::RandomState;

use crate::id::WeakId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A character's core attributes.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Attributes {
    strength: u8,
}

impl Attributes {
    pub fn new() -> Self {
        Self { strength: 0 }
    }

    pub fn get(&self, attribute: Attribute) -> u8 {
        match attribute {
            Attribute::STRENGTH => self.strength,
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, attribute: Attribute, value: u8) {
        match attribute {
            Attribute::STRENGTH => self.strength = value,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attribute(NonZeroU8);

impl Attribute {
    pub const STRENGTH: Self = Self(NonZeroU8::new(1).unwrap());
}

/// A unique identifer for a [`Trait`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraitId(WeakId<u32>);

/// A set of traits that a character may obtain.
pub struct Traits {
    traits: HashMap<TraitId, Trait, RandomState>,
}

impl Traits {
    pub fn new() -> Self {
        Self {
            traits: HashMap::with_hasher(RandomState::new()),
        }
    }
}

pub struct Trait {
    pub id: TraitId,
}
