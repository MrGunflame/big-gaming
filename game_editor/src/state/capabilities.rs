use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities(u8);

impl Capabilities {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(1 << 1);

    #[inline]
    pub fn read(self) -> bool {
        (self & Self::READ) != Self::NONE
    }

    #[inline]
    pub fn write(self) -> bool {
        (self & Self::WRITE) != Self::NONE
    }
}

impl BitAnd for Capabilities {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Capabilities {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for Capabilities {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Capabilities {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
