use std::time::Duration;

use bytes::{Buf, BufMut};

use super::{Decode, Encode};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Timestamp(u32);

impl Timestamp {
    #[inline]
    pub fn new(start: Duration) -> Self {
        Self::from_micros(start.as_micros())
    }

    #[inline]
    pub fn to_bits(self) -> u32 {
        self.0
    }

    #[inline]
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    #[inline]
    pub fn to_duration(self) -> Duration {
        Duration::from_micros(self.0 as u64)
    }

    #[inline]
    fn from_micros(n: u128) -> Self {
        Self(n as u32)
    }
}

impl Encode for Timestamp {
    type Error = <u32 as Encode>::Error;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for Timestamp {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

impl From<Timestamp> for Duration {
    #[inline]
    fn from(value: Timestamp) -> Self {
        value.to_duration()
    }
}
