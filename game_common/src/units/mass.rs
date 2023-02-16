use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Mass(u32);

impl Mass {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(u32::MAX);

    /// Creates a new zero `Mass`.
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_grams(g: u32) -> Self {
        Self(g)
    }

    #[inline]
    pub const fn from_kilograms(kg: u32) -> Self {
        if cfg!(debug_assertions) {
            Self(kg * 1000)
        } else {
            Self(kg.saturating_mul(1000))
        }
    }

    #[inline]
    pub const fn to_grams(self) -> u32 {
        self.0
    }

    #[inline]
    pub fn to_kilograms_f32(self) -> f32 {
        self.0 as f32 / 1000.0
    }

    #[inline]
    pub const fn checked_add(self, rhs: Mass) -> Option<Mass> {
        match self.0.checked_add(rhs.0) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    #[inline]
    pub const fn checked_sub(self, rhs: Mass) -> Option<Mass> {
        match self.0.checked_sub(rhs.0) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    #[inline]
    pub const fn checked_mul(self, rhs: u32) -> Option<Mass> {
        match self.0.checked_mul(rhs) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }
}

impl const Add for Mass {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl const AddAssign for Mass {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl const Sub for Mass {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl const SubAssign for Mass {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl const Mul<u32> for Mass {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl const MulAssign<u32> for Mass {
    #[inline]
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}
