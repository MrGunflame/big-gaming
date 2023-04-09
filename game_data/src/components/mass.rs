use bytes::{Buf, BufMut};
use game_common::units::Mass;

use crate::{Decode, Encode};

impl Encode for Mass {
    #[inline]
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.to_grams().encode(buf);
    }
}

impl Decode for Mass {
    type Error = std::io::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let g = u32::decode(buf)?;
        Ok(Self::from_grams(g))
    }
}
