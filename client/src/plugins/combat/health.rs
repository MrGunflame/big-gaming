use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::ops::{Div, Sub, SubAssign};

use bevy::prelude::Component;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The health value of an actor.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Health(u32);

impl Health {
    #[inline]
    pub const fn new(val: u32) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl Display for Health {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Sub<u32> for Health {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl SubAssign<u32> for Health {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Hash, Component)]
pub struct MaxHealth(u32);

impl MaxHealth {
    #[inline]
    pub const fn new(val: u32) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl PartialEq<Health> for MaxHealth {
    #[inline]
    fn eq(&self, other: &Health) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd<Health> for MaxHealth {
    #[inline]
    fn partial_cmp(&self, other: &Health) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for MaxHealth {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Div<MaxHealth> for Health {
    type Output = f32;

    fn div(self, rhs: MaxHealth) -> Self::Output {
        self.0 as f32 / rhs.0 as f32
    }
}
