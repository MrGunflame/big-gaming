use std::convert::Infallible;

use bytes::BufMut;
use game_common::components::properties::PropertyValue;

use super::Encode;

impl Encode for PropertyValue {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        match self {
            Self::None => (),
            Self::Entity(_) => todo!(),
            Self::I32(val) => val.encode(buf)?,
            Self::I64(val) => val.encode(buf)?,
            Self::Bytes(bytes) => {
                for b in bytes.iter() {
                    b.encode(&mut buf)?;
                }
            }
        }

        Ok(())
    }
}

// impl Decode for PropertyValue {
//     type Error = EofError;

//     fn decode<B>(buf: B) -> Result<Self, Self::Error>
//     where
//         B: Buf,
//     {
//     }
// }
