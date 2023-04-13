use std::fmt::{self, Display, Formatter, LowerHex};

use bytes::{Buf, BufMut};
use game_common::units::Mass;

use crate::record::RecordId;
use crate::{Decode, Encode, EofError};

#[derive(Clone, Debug)]
pub struct ItemRecord {
    pub id: RecordId,
    pub name: String,
    pub mass: Mass,
    pub value: u64,
}

impl Encode for ItemRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.name.encode(&mut buf);
        self.mass.encode(&mut buf);
    }
}

impl Decode for ItemRecord {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordId::decode(&mut buf)?;
        let name = String::decode(&mut buf)?;
        let mass = Mass::decode(&mut buf)?;
        let value = u64::decode(&mut buf)?;

        Ok(Self {
            id,
            name,
            mass,
            value,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

impl Encode for ItemId {
    #[inline]
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for ItemId {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

impl Display for ItemId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}
