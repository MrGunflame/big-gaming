use bytes::{Buf, BufMut};

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum CompressionScheme {
    #[default]
    None,
}

impl Encode for CompressionScheme {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let b: u8 = match self {
            Self::None => 0,
        };

        b.encode(buf);
    }
}

impl Decode for CompressionScheme {
    type Error = ();

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let b = u8::decode(buf)?;

        match b {
            0 => Ok(Self::None),
            _ => Err(()),
        }
    }
}
