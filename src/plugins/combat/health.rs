use std::ops::{Sub, SubAssign};

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

/// The health value of an actor.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Component,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Health(u32);

impl Health {
    #[inline]
    pub const fn new(val: u32) -> Self {
        Self(val)
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
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
