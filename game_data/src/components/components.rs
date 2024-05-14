use bytes::{Buf, BufMut};
use game_common::record::RecordReference;
use game_common::reflection::{
    self, ComponentDescriptor, Field, FieldIndex, FieldKind, FloatField, IntegerField,
};
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode, EofError, StringError};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ComponentRecordError {
    #[error("failed to decode component description: {0}")]
    Description(<String as Decode>::Error),
    #[error("failed to decode component script: {0}")]
    Script(<Uri as Decode>::Error),
    #[error("failed to decode component descriptor: {0}")]
    Descriptor(<ComponentDescriptor as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ComponentRecord {
    pub description: String,
    pub descriptor: ComponentDescriptor,
}

impl Encode for ComponentRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.description.encode(&mut buf);
        self.descriptor.encode(&mut buf);
    }
}

impl Decode for ComponentRecord {
    type Error = ComponentRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let description = String::decode(&mut buf).map_err(ComponentRecordError::Description)?;
        let descriptor =
            ComponentDescriptor::decode(&mut buf).map_err(ComponentRecordError::Descriptor)?;

        Ok(Self {
            description,
            descriptor,
        })
    }
}

/// The value of a component.
#[derive(Clone, Debug)]
pub struct ComponentValue {
    pub id: RecordReference,
    pub bytes: Vec<u8>,
}

impl ComponentValue {
    pub const fn new(id: RecordReference) -> Self {
        Self {
            id,
            bytes: Vec::new(),
        }
    }
}

impl Encode for ComponentValue {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.bytes.encode(&mut buf);
    }
}

impl Decode for ComponentValue {
    type Error = ComponentValueError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordReference::decode(&mut buf).map_err(ComponentValueError::Id)?;
        let bytes = Vec::decode(&mut buf).map_err(ComponentValueError::Value)?;

        Ok(Self { id, bytes })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ComponentValueError {
    #[error("failed to decode component id: {0}")]
    Id(<RecordReference as Decode>::Error),
    #[error("failed to decode component value: {0}")]
    Value(<Vec<u8> as Decode>::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum DecodeComponentDescriptorError {
    #[error("eof: {0}")]
    Eof(#[from] EofError),
    #[error("invalid string: {0}")]
    InvalidString(#[from] StringError),
    #[error("bad component descriptor: {0}")]
    BadDescriptor(reflection::Error),
    #[error("invalid field kind: {0}")]
    InvalidFieldKind(u8),
}

impl Encode for ComponentDescriptor {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let len_fields = self.fields().len() as u16;
        len_fields.encode(&mut buf);
        for field in self.fields() {
            field.encode(&mut buf);
        }

        let len_root = self.root().len() as u16;
        len_root.encode(&mut buf);
        for index in self.root() {
            index.into_raw().encode(&mut buf);
        }
    }
}

impl Decode for ComponentDescriptor {
    type Error = DecodeComponentDescriptorError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len_fields = u16::decode(&mut buf)?;
        let mut fields = Vec::new();
        for _ in 0..len_fields {
            let field = Field::decode(&mut buf)?;
            fields.push(field);
        }

        let len_root = u16::decode(&mut buf)?;
        let mut root = Vec::new();
        for _ in 0..len_root {
            let index = FieldIndex::from_raw(u16::decode(&mut buf)?);
            root.push(index);
        }

        Self::new(fields, root).map_err(DecodeComponentDescriptorError::BadDescriptor)
    }
}

impl Encode for Field {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.name.encode(&mut buf);
        match &self.kind {
            FieldKind::Int(field) => {
                0u8.encode(&mut buf);
                field.encode(buf);
            }
            FieldKind::Float(field) => {
                1u8.encode(&mut buf);
                field.encode(buf);
            }
            FieldKind::Struct(indices) => {
                2u8.encode(&mut buf);
                let len = indices.len() as u16;
                len.encode(&mut buf);
                for index in indices {
                    index.into_raw().encode(&mut buf);
                }
            }
        }
    }
}

impl Decode for Field {
    type Error = DecodeComponentDescriptorError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let name = String::decode(&mut buf)?;
        let tag = u8::decode(&mut buf)?;
        let kind = match tag {
            0 => FieldKind::Int(IntegerField::decode(buf)?),
            1 => FieldKind::Float(FloatField::decode(buf)?),
            2 => {
                let mut indices = Vec::new();
                let len = u16::decode(&mut buf)?;
                for _ in 0..len {
                    let index = FieldIndex::from_raw(u16::decode(&mut buf)?);
                    indices.push(index);
                }
                FieldKind::Struct(indices)
            }
            _ => return Err(DecodeComponentDescriptorError::InvalidFieldKind(tag)),
        };

        Ok(Self { name, kind })
    }
}

impl Encode for IntegerField {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.bits.encode(&mut buf);
    }
}

impl Decode for IntegerField {
    type Error = DecodeComponentDescriptorError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let bits = u8::decode(&mut buf)?;
        Ok(Self {
            bits,
            is_signed: false,
            min: None,
            max: None,
        })
    }
}

impl Encode for FloatField {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.bits.encode(&mut buf);
    }
}

impl Decode for FloatField {
    type Error = DecodeComponentDescriptorError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let bits = u8::decode(&mut buf)?;
        Ok(Self { bits })
    }
}
