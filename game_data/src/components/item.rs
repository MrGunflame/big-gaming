use bytes::{Buf, BufMut};
use game_common::units::Mass;

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub mass: Mass,
}

impl Encode for Item {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.name.encode(&mut buf);
        self.mass.encode(&mut buf);
    }
}

impl Decode for Item {
    type Error = std::io::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ItemId::decode(&mut buf)?;
        let name = String::decode(&mut buf)?;
        let mass = Mass::decode(&mut buf)?;

        Ok(Self { id, name, mass })
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
