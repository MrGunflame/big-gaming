use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::net::ServerEntity;
use game_common::record::RecordReference;

use super::{Decode, Encode, EofError, Error};

#[derive(Clone, Debug)]
pub struct ComponentAdd {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
    pub bytes: Vec<u8>,
}

impl Encode for ComponentAdd {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.entity.encode(&mut buf)?;
        self.component_id.encode(&mut buf)?;
        (self.bytes.len() as u64).encode(&mut buf)?;
        buf.put_slice(&self.bytes);
        Ok(())
    }
}

impl Decode for ComponentAdd {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let entity = ServerEntity::decode(&mut buf)?;
        let component_id = RecordReference::decode(&mut buf)?;

        let len = u64::decode(&mut buf)?;
        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(u8::decode(&mut buf)?);
        }

        Ok(Self {
            entity,
            component_id,
            bytes,
        })
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ComponentRemove {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
}

#[derive(Clone, Debug)]
pub struct ComponentUpdate {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
    pub bytes: Vec<u8>,
}

impl Encode for ComponentUpdate {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.entity.encode(&mut buf)?;
        self.component_id.encode(&mut buf)?;

        (self.bytes.len() as u64).encode(&mut buf)?;
        buf.put_slice(&self.bytes);
        Ok(())
    }
}

impl Decode for ComponentUpdate {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let entity = ServerEntity::decode(&mut buf)?;
        let component_id = RecordReference::decode(&mut buf)?;

        let len = u64::decode(&mut buf)?;
        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(u8::decode(&mut buf)?);
        }

        Ok(Self {
            entity,
            component_id,
            bytes,
        })
    }
}
