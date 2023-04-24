use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::components::components::{Component, Components, RecordReference};

use super::{Decode, Encode, EofError};

impl Encode for Components {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        (self.len() as u64).encode(&mut buf)?;
        for (id, val) in self.iter() {
            id.encode(&mut buf)?;

            let bytes = val.as_bytes();
            (bytes.len() as u64).encode(&mut buf)?;
            buf.put_slice(bytes);
        }

        Ok(())
    }
}

impl Decode for Components {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len = u64::decode(&mut buf)?;

        let mut components = Self::new();
        for _ in 0..len {
            let id = RecordReference::decode(&mut buf)?;

            let len = u64::decode(&mut buf)? as usize;

            if buf.remaining() < len {
                return Err(EofError {
                    expected: len,
                    found: buf.remaining(),
                });
            }

            let mut bytes = vec![0; len];
            buf.copy_to_slice(&mut bytes);

            components.insert(id, Component { bytes });
        }

        Ok(components)
    }
}
