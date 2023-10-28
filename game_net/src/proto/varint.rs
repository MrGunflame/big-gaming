use std::convert::Infallible;

use bytes::BufMut;

use super::{Decode, Encode, EofError};

const CONTINUE_BIT: u8 = 1 << 7;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarInt<T>(pub T);

macro_rules! impl_varint {
    ($t:ty) => {
        impl Encode for VarInt<$t> {
            type Error = Infallible;

            fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
            where
                B: BufMut,
            {
                let mut n = self.0;

                loop {
                    let byte = n & (u8::MAX as $t);
                    let mut byte = byte as u8 & !CONTINUE_BIT;

                    n >>= 7;
                    if n != 0 {
                        byte |= CONTINUE_BIT;
                    }

                    buf.put_u8(byte);

                    if n == 0 {
                        break;
                    }
                }

                Ok(())
            }
        }

        impl Decode for VarInt<$t> {
            type Error = EofError;

            fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
            where
                B: bytes::Buf,
            {
                let mut n = 0;
                let mut shift = 0;

                loop {
                    let byte = u8::decode(&mut buf)?;

                    if shift == u64::BITS - 1 {
                        return Ok(Self(<$t>::MAX));
                    }

                    n += ((byte & !CONTINUE_BIT) as $t) << shift;
                    shift += 7;

                    if byte & CONTINUE_BIT == 0 {
                        return Ok(Self(n));
                    }
                }
            }
        }

        impl From<$t> for VarInt<$t> {
            #[inline]
            fn from(value: $t) -> Self {
                Self(value)
            }
        }

        impl From<VarInt<$t>> for $t {
            #[inline]
            fn from(value: VarInt<$t>) -> Self {
                value.0
            }
        }
    };
}

impl_varint!(u16);
impl_varint!(u32);
impl_varint!(u64);

#[cfg(test)]
mod tests {
    use crate::proto::{Decode, Encode};

    use super::VarInt;

    #[test]
    fn varint_u64_max() {
        let mut buf = Vec::new();
        VarInt(u64::MAX).encode(&mut buf).unwrap();

        dbg!(&buf);

        assert_eq!(VarInt::<u64>::decode(&buf[..]).unwrap().0, u64::MAX);
    }
}
