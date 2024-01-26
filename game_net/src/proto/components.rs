use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::components::components::{Components, RawComponent};
use game_common::net::ServerEntity;
use game_common::record::RecordReference;
use game_wasm::encoding::{decode_fields, encode_fields, Field};

use super::varint::VarInt;
use super::{Decode, Encode, EofError, Error};

impl Encode for RawComponent {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        (self.as_bytes().len() as u64).encode(&mut buf)?;
        buf.put_slice(self.as_bytes());
        (self.fields().len() as u64).encode(&mut buf)?;
        buf.put_slice(&encode_fields(self.fields()));
        Ok(())
    }
}

impl Decode for RawComponent {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let data_len = u64::decode(&mut buf)?;
        let mut data = Vec::new();
        for _ in 0..data_len {
            data.push(u8::decode(&mut buf)?);
        }

        let fields_len = u64::decode(&mut buf)? as usize * Field::ENCODED_SIZE;
        let mut fields = Vec::new();
        for _ in 0..fields_len {
            fields.push(u8::decode(&mut buf)?);
        }
        let fields = decode_fields(&fields);

        Ok(Self::new(data, fields))
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ComponentAdd {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
    pub component: RawComponent,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ComponentRemove {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ComponentUpdate {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
    pub component: RawComponent,
}

impl Encode for Components {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        VarInt::<u64>(self.len() as u64).encode(&mut buf)?;
        for (id, component) in self.iter() {
            id.encode(&mut buf)?;
            component.encode(&mut buf)?;
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
        let len = VarInt::<u64>::decode(&mut buf)?.0;

        let mut components = Components::new();
        for _ in 0..len {
            let id = RecordReference::decode(&mut buf)?;
            let component = RawComponent::decode(&mut buf)?;
            components.insert(id, component);
        }

        Ok(components)
    }
}
