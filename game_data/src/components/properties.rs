use bytes::{Buf, BufMut};
use game_common::components::properties::PropertyKind;
use thiserror::Error;

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct PropertyRecord {
    kind: PropertyKind,
}

impl Encode for PropertyRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.kind.encode(&mut buf);
    }
}

impl Decode for PropertyRecord {
    type Error = <PropertyKind as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let kind = PropertyKind::decode(buf)?;

        Ok(Self { kind })
    }
}

impl Encode for PropertyKind {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let byte: u8 = match self {
            PropertyKind::None => 0,
            PropertyKind::I32 => 1,
            PropertyKind::I64 => 2,
            PropertyKind::Bytes => 3,
            PropertyKind::Entity => 4,
        };

        byte.encode(buf);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum PropertyKindError {
    #[error("failed to decode property kind flag: {0}")]
    Byte(<u8 as Decode>::Error),
    #[error("invalid property kind: {0}")]
    InvalidKind(u8),
}

impl Decode for PropertyKind {
    type Error = PropertyKindError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let byte = u8::decode(buf).map_err(PropertyKindError::Byte)?;

        match byte {
            0 => Ok(Self::None),
            1 => Ok(Self::I32),
            2 => Ok(Self::I64),
            3 => Ok(Self::Bytes),
            4 => Ok(Self::Entity),
            _ => Err(PropertyKindError::InvalidKind(byte)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Sets the property to the given value, overwriting any existing previous formula.
    Set,
    /// Adds the given value to the previous value.
    Add,
    Mul,
}
