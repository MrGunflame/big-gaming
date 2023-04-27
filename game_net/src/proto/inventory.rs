use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::components::items::ItemId;
use game_common::record::RecordReference;

use super::{Decode, Encode, EofError};

impl Encode for ItemId {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ItemId {
    type Error = EofError;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        RecordReference::decode(buf).map(Self)
    }
}
