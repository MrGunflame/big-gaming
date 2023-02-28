use std::convert::Infallible;

use bytes::{Buf, BufMut};

use super::{Decode, Encode, Error};

#[derive(Clone, Debug, Encode, Decode)]
pub struct Shutdown {
    pub reason: ShutdownReason,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShutdownReason(u8);

impl Encode for ShutdownReason {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ShutdownReason {
    type Error = Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        // TODO: Error handling
        Ok(Self(u8::decode(buf)?))
    }
}
