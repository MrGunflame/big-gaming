use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::{Decode, Encode};

const CONTINUE_BIT: u8 = 1 << 7;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarU64(pub u64);

impl Encode for VarU64 {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let mut val = self.0;

        loop {
            let mut byte = val as u8 & !CONTINUE_BIT;

            val >>= 7;
            if val != 0 {
                byte |= CONTINUE_BIT;
            }

            buf.put_u8(byte);

            if val == 0 {
                return;
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum VarIntError {
    #[error("failed to decode byte: {0}")]
    Byte(<u8 as Decode>::Error),
    #[error("varint too large")]
    Overflow,
}

impl Decode for VarU64 {
    type Error = VarIntError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut val = 0;
        let mut shift = 0;

        loop {
            let byte = u8::decode(&mut buf).map_err(VarIntError::Byte)?;

            if shift == u64::BITS - 1 {
                return Err(VarIntError::Overflow);
            }

            val += ((byte & !CONTINUE_BIT) as u64) << shift;

            // If the continue bit is not set the integer has ended.
            if byte & CONTINUE_BIT == 0 {
                return Ok(Self(val));
            }

            shift += 7;
        }
    }
}
