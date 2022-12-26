use std::convert::Infallible;

use bytes::{Buf, BufMut};

use crate::components::combat::{Health, Resistance};

use super::{Decode, Encode};

impl Encode for Health {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.health.encode(&mut buf);
        self.max_health.encode(&mut buf);
    }
}

impl Decode for Health {
    type Error = Infallible;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let health = u32::decode(&mut buf)?;
        let max_health = u32::decode(&mut buf)?;

        Ok(Self { health, max_health })
    }
}

impl Encode for Resistance {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_u32(self.to_u32());
    }
}

impl Decode for Resistance {
    type Error = Infallible;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let value = buf.get_u32();
        Ok(Resistance::new(value))
    }
}
