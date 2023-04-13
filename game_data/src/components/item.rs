use std::fmt::{self, Display, Formatter, LowerHex};

use bytes::{Buf, BufMut};
use game_common::units::Mass;
use thiserror::Error;

use crate::record::RecordId;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ItemRecordError {
    #[error("failed to decode record id: {0}")]
    Id(<RecordId as Decode>::Error),
    #[error("failed to decode record name: {0}")]
    Name(<String as Decode>::Error),
    #[error("failed to decode item mass: {0}")]
    Mass(<Mass as Decode>::Error),
    #[error("failed to decode item value: {0}")]
    Value(<u64 as Decode>::Error),
}

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
        self.value.encode(&mut buf);
    }
}

impl Decode for ItemRecord {
    type Error = ItemRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordId::decode(&mut buf).map_err(ItemRecordError::Id)?;
        let name = String::decode(&mut buf).map_err(ItemRecordError::Name)?;
        let mass = Mass::decode(&mut buf).map_err(ItemRecordError::Mass)?;
        let value = u64::decode(&mut buf).map_err(ItemRecordError::Value)?;

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
