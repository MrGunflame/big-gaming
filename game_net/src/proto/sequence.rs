use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use bytes::{Buf, BufMut};

use crate::serial;

use super::{Decode, Encode};

const BITS: usize = 31;

/// A sequence number
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Sequence(u32);

impl Sequence {
    #[inline]
    pub fn new(n: u32) -> Self {
        Self(n)
    }

    #[inline]
    pub fn to_bits(self) -> u32 {
        self.0
    }

    #[inline]
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
}

impl Default for Sequence {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl Add<u32> for Sequence {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u32) -> Self::Output {
        Self(serial::add::<BITS, _>(self.0, rhs))
    }
}

impl AddAssign<u32> for Sequence {
    #[inline]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

impl Sub<u32> for Sequence {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u32) -> Self::Output {
        Self(serial::sub::<BITS, _>(self.0, rhs))
    }
}

impl SubAssign<u32> for Sequence {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

impl PartialOrd for Sequence {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sequence {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        serial::cmp::<BITS>(self.0, other.0)
    }
}

impl Encode for Sequence {
    type Error = <u32 as Encode>::Error;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for Sequence {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}
