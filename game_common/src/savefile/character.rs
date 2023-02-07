use std::convert::Infallible;

use bytes::{Buf, BufMut};

use crate::components::character::{Attribute, Attributes};

use super::{Decode, Encode};

impl Encode for Attributes {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_u8(self.get(Attribute::STRENGTH));
    }
}

impl Decode for Attributes {
    type Error = Infallible;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut attributes = Attributes::new();
        attributes.set(Attribute::STRENGTH, buf.get_u8());
        Ok(attributes)
    }
}
