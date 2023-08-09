use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub, SubAssign};

// FIXME: Ord impl should wrap
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ControlFrame(pub u16);

impl ControlFrame {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl Add for ControlFrame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_add(rhs.0))
    }
}

impl AddAssign for ControlFrame {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for ControlFrame {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl SubAssign for ControlFrame {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Add<u16> for ControlFrame {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl AddAssign<u16> for ControlFrame {
    fn add_assign(&mut self, rhs: u16) {
        *self = *self + rhs;
    }
}

impl Sub<u16> for ControlFrame {
    type Output = Self;

    fn sub(self, rhs: u16) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl SubAssign<u16> for ControlFrame {
    fn sub_assign(&mut self, rhs: u16) {
        *self = *self - rhs;
    }
}

impl Ord for ControlFrame {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = self.0;
        let rhs = other.0;

        if lhs == rhs {
            return Ordering::Equal;
        }

        // Based on the serial impl from `game_net/src(serial.rs`. (RFC 1982)
        if (lhs < rhs && rhs.wrapping_sub(lhs) < 1 << (16 - 1))
            || (lhs > rhs && lhs.wrapping_sub(rhs) > 1 << (16 - 1))
        {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for ControlFrame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
