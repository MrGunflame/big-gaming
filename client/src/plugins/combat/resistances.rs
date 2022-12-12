use std::borrow::Borrow;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use bevy::prelude::Component;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Component)]
pub struct Resistances {
    classes: HashMap<ResistanceId, Resistance>,
}

impl Resistances {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    pub fn get<T>(&self, class: T) -> Option<Resistance>
    where
        T: Borrow<ResistanceId>,
    {
        self.classes.get(class.borrow()).copied()
    }

    pub fn get_mut<T>(&mut self, class: T) -> Option<&mut Resistance>
    where
        T: Borrow<ResistanceId>,
    {
        self.classes.get_mut(class.borrow())
    }

    /// Adds `value` to the specified resistance. Returns the new resistance.
    pub fn add<T>(&mut self, class: T, value: u32) -> Option<Resistance>
    where
        T: Borrow<ResistanceId>,
    {
        match self.get_mut(class.borrow()) {
            Some(res) => {
                *res += value;
                Some(*res)
            }
            None => None,
        }
    }

    pub fn sub<T>(&mut self, class: T, value: u32) -> Option<Resistance>
    where
        T: Borrow<ResistanceId>,
    {
        match self.get_mut(class.borrow()) {
            Some(res) => {
                *res -= value;
                Some(*res)
            }
            None => None,
        }
    }
}

/// A globally unique identifier for a [`Resistance`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ResistanceId(NonZeroU32);

impl ResistanceId {
    pub const BALLISTIC: Self = Self(NonZeroU32::new(2).unwrap());
    pub const ENERGY: Self = Self(NonZeroU32::new(3).unwrap());
}

/// A damage resistance value.
///
/// A `Resistance` reduces the incoming received damage. A value should be kept for every class of
/// damage.
///
/// `Resistance` implements [`Add`] and [`Sub`], saturating at the bounds of `u32::MAX` instead of
/// overflowing or panicking.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Resistance(u32);

impl Resistance {
    /// Creates a new `Resistance` with the given value.
    #[inline]
    pub const fn new(val: u32) -> Self {
        Self(val)
    }
}

impl Add<u32> for Resistance {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl AddAssign<u32> for Resistance {
    #[inline]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

impl Sub<u32> for Resistance {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl SubAssign<u32> for Resistance {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}
