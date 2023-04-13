use std::fmt::{self, Display, Formatter, LowerHex};

use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::components::item::ItemRecord;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RecordId(pub u32);

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub id: RecordId,
    pub name: String,
    pub body: RecordBody,
}

#[derive(Clone, Debug)]
pub enum RecordBody {
    Item(ItemRecord),
}

impl RecordBody {
    pub const fn kind(&self) -> RecordKind {
        match self {
            Self::Item(_) => RecordKind::Item,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    Item,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum RecordKindError {
    #[error("failed to decode record kind byte: {0}")]
    Byte(<u8 as Decode>::Error),
    #[error("found invalid record kind: {0}")]
    InvalidKind(u8),
}

impl Encode for RecordKind {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let byte: u8 = match self {
            Self::Item => 1,
        };

        byte.encode(buf);
    }
}

impl Decode for RecordKind {
    type Error = RecordKindError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let byte = u8::decode(buf).map_err(RecordKindError::Byte)?;

        match byte {
            1 => Ok(Self::Item),
            _ => Err(RecordKindError::InvalidKind(byte)),
        }
    }
}

impl Encode for RecordId {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for RecordId {
    type Error = <u32 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

impl Encode for Record {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.name.encode(&mut buf);

        match &self.body {
            RecordBody::Item(item) => {
                1u8.encode(&mut buf);
                item.encode(&mut buf);
            }
        };
    }
}

impl Decode for Record {
    type Error = RecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordId::decode(&mut buf).map_err(RecordError::Id)?;
        let name = String::decode(&mut buf).map_err(RecordError::Name)?;
        let kind = RecordKind::decode(&mut buf).map_err(RecordError::Kind)?;

        let body = match kind {
            RecordKind::Item => {
                let item = ItemRecord::decode(&mut buf)?;
                RecordBody::Item(item)
            }
        };

        Ok(Self { id, name, body })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum RecordError {
    #[error("failed to decode record id: {0}")]
    Id(<RecordId as Decode>::Error),
    #[error("failed to decode record name: {0}")]
    Name(<String as Decode>::Error),
    #[error("failed to decode record kind: {0}")]
    Kind(<RecordKind as Decode>::Error),
    #[error("failed to decode item record: {0}")]
    Item(#[from] <ItemRecord as Decode>::Error),
}
