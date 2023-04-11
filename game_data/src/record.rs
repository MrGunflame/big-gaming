use std::fmt::{self, Display, Formatter, LowerHex};

use bytes::{Buf, BufMut};

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
