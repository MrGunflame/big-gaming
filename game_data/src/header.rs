use bytes::{Buf, BufMut};
use game_common::module::ModuleId;

use crate::{Decode, Encode};

pub const MAGIC: [u8; 4] = [0, 0, 0, 0];

#[derive(Clone, Debug, Default)]
pub struct Header {
    // magic outlined
    pub version: u8,

    pub id: ModuleId,

    pub items: u32,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(&MAGIC);

        self.version.encode(&mut buf);
        self.id.encode(&mut buf);
        self.items.encode(&mut buf);
    }
}

impl Decode for Header {
    type Error = std::io::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let magic = u32::decode(&mut buf)?;
        assert!(magic == u32::from_ne_bytes(MAGIC));

        let version = u8::decode(&mut buf)?;
        let items = u32::decode(&mut buf)?;
        let id = ModuleId::decode(&mut buf)?;

        Ok(Self { version, items, id })
    }
}

impl Encode for ModuleId {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(&self.into_bytes());
    }
}

impl Decode for ModuleId {
    type Error = std::io::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut bytes = [0; 16];
        buf.copy_to_slice(&mut bytes);
        Ok(Self::from_bytes(bytes))
    }
}
