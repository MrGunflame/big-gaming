use bytes::{Buf, BufMut};
use game_common::components::combat::{Resistance, Resistances};

use crate::{Decode, Encode};

impl Encode for Resistances {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
    }
}

impl Encode for Resistance {
    #[inline]
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for Resistance {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}
