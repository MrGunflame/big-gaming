use std::ops::{Add, AddAssign, Sub, SubAssign};

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Component)]
pub struct Resistances {}

impl Resistances {
    pub fn new() -> Self {
        Self {}
    }
}

/// A damage resistance value.
///
/// A `Resistance` reduces the incoming received damage. A value should be kept for every class of
/// damage.
///
/// `Resistance` implements [`Add`] and [`Sub`], saturating at the bounds of `u32::MAX` instead of
/// overflowing or panicking.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
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
