use bytes::{Buf, BufMut};
use game_common::components::combat::{Damage, DamageClass, Health, Resistance};
use game_common::id::WeakId;

use super::{Decode, Encode};

impl Encode for Health {
    type Error = <u32 as Encode>::Error;

    #[inline]
    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.health.encode(&mut buf)?;
        self.max_health.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for Health {
    type Error = <u32 as Decode>::Error;

    #[inline]
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
    type Error = <u32 as Encode>::Error;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
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

impl Encode for DamageClass {
    type Error = <u32 as Encode>::Error;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for DamageClass {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(WeakId).map(Self)
    }
}

impl Encode for Damage {
    type Error = <u32 as Encode>::Error;

    #[inline]
    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.class.encode(&mut buf)?;
        self.amount.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for Damage {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let class = DamageClass::decode(&mut buf)?;
        let amount = u32::decode(&mut buf)?;

        Ok(Self { class, amount })
    }
}
