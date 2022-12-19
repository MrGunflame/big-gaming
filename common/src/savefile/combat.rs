use std::convert::Infallible;
use std::io::{self, Write};

use bytes::{Buf, BufMut};

use crate::components::combat::{Damage, Health, Resistance, ResistanceId};

use super::{Decode, Encode};

impl Encode for Health {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_u32(self.health);
        buf.put_u32(self.max_health);
    }
}

impl Decode for Health {
    type Error = Infallible;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let health = buf.get_u32();
        let max_health = buf.get_u32();

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
