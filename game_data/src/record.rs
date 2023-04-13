use std::fmt::{self, Display, Formatter, LowerHex};

use bytes::{Buf, BufMut};

use crate::components::item::ItemRecord;
use crate::{Decode, Encode, EofError};

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
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordId::decode(&mut buf)?;
        let name = String::decode(&mut buf)?;
        let kind = u8::decode(&mut buf)?;

        let body = match kind {
            0 => {
                let item = ItemRecord::decode(&mut buf)?;
                RecordBody::Item(item)
            }
            _ => panic!("bad record type"),
        };

        Ok(Self { id, name, body })
    }
}
