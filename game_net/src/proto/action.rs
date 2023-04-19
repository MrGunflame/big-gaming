use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::components::actions::ActionId;
use game_common::components::components::RecordReference;

use super::{Decode, Encode, EofError};

impl Encode for ActionId {
    type Error = Infallible;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ActionId {
    type Error = EofError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        RecordReference::decode(buf).map(Self)
    }
}
