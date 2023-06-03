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
    endurance: u8,
}

impl Attributes {
    #[inline]
    pub fn new() -> Self {
        Self {
            strength: 0,
            endurance: 0,
        }
    }

    #[inline]
    pub fn get(&self, attribute: Attribute) -> u8 {
        match attribute {
            a if a == Attribute::STRENGTH => self.strength,
            a if a == Attribute::ENDURANCE => self.endurance,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn set(&mut self, attribute: Attribute, value: u8) {
        match attribute {
            a if a == Attribute::STRENGTH => self.strength = value,
            a if a == Attribute::ENDURANCE => self.endurance = value,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attribute(NonZeroU8);

impl Attribute {
    pub const STRENGTH: Self = Self(unsafe { NonZeroU8::new_unchecked(1) });
    pub const ENDURANCE: Self = Self(unsafe { NonZeroU8::new_unchecked(2) });
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
