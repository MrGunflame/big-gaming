//! The binary format used for savegames.
//!

use bytes::{Buf, BufMut};
mod combat;
mod items;

pub struct Error {}

pub trait Encode {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut;
}

pub trait Decode: Sized {
    type Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf;
}
