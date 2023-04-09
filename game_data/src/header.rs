use bytes::{Buf, BufMut};

use crate::{Decode, Encode};

pub const MAGIC: [u8; 4] = [0, 0, 0, 0];

#[derive(Clone, Debug)]
pub struct Header {
    // magic outlined
    pub version: u8,

    pub items: u32,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(&MAGIC);
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

        Ok(Self { version, items })
    }
}
