use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};

use super::{Decode, Encode, EofError};

impl Encode for RecordReference {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.module.encode(&mut buf)?;
        self.record.0.encode(&mut buf)?;

        Ok(())
    }
}

impl Decode for RecordReference {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let module = ModuleId::decode(&mut buf)?;
        let record = u32::decode(&mut buf)?;

        Ok(Self {
            module,
            record: RecordId(record),
        })
    }
}

impl Encode for ModuleId {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        buf.put_slice(&self.into_bytes());
        Ok(())
    }
}

impl Decode for ModuleId {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        if buf.remaining() < 16 {
            return Err(EofError {
                expected: 16,
                found: buf.remaining(),
            });
        }

        let mut bytes = [0; 16];
        buf.copy_to_slice(&mut bytes);

        Ok(Self::from_bytes(bytes))
    }
}
