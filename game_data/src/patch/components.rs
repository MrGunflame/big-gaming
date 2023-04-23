use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::record::RecordReference;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum OperationError {
    #[error("failed to decode operation type: {0}")]
    Byte(<u8 as Decode>::Error),
    #[error("invalid operation kind: {0}")]
    InvalidKind(u8),
    #[error("failed to decode add operation: {0}")]
    Add(<AddOperation as Decode>::Error),
    #[error("failed to decode remove operation: {0}")]
    Remove(<RemoveOperation as Decode>::Error),
}

/// Patch a component on a record.
#[derive(Clone, Debug)]
pub enum Operation {
    /// Add a new component.
    Add(AddOperation),
    Remove(RemoveOperation),
}

impl Operation {
    #[inline]
    pub const fn is_add(&self) -> bool {
        matches!(self, Self::Add(_))
    }

    #[inline]
    pub const fn is_remove(&self) -> bool {
        matches!(self, Self::Remove(_))
    }
}

impl Encode for Operation {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        match self {
            Self::Add(op) => {
                1u8.encode(&mut buf);
                op.encode(buf)
            }
            Self::Remove(op) => {
                2u8.encode(&mut buf);
                op.encode(buf)
            }
        }
    }
}

impl Decode for Operation {
    type Error = OperationError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let kind = u8::decode(&mut buf).map_err(OperationError::Byte)?;

        match kind {
            1u8 => {
                let op = AddOperation::decode(buf).map_err(OperationError::Add)?;
                Ok(Self::Add(op))
            }
            2u8 => {
                let op = RemoveOperation::decode(buf).map_err(OperationError::Remove)?;
                Ok(Self::Remove(op))
            }
            _ => Err(OperationError::InvalidKind(kind)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AddOperation {
    pub id: RecordReference,
}

impl Encode for AddOperation {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
    }
}

impl Decode for AddOperation {
    type Error = <RecordReference as Decode>::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordReference::decode(&mut buf)?;

        Ok(Self { id })
    }
}

#[derive(Clone, Debug)]
pub struct RemoveOperation {
    pub id: RecordReference,
}

impl Encode for RemoveOperation {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
    }
}

impl Decode for RemoveOperation {
    type Error = <RecordReference as Decode>::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordReference::decode(&mut buf)?;

        Ok(Self { id })
    }
}
