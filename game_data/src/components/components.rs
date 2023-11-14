use bytes::{Buf, BufMut};
use game_common::record::RecordReference;
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ComponentRecordError {
    #[error("failed to decode component description: {0}")]
    Description(<String as Decode>::Error),
    #[error("failed to decode component script: {0}")]
    Script(<Uri as Decode>::Error),
    #[error("failed to decode component actions: {0}")]
    Actions(<Vec<RecordReference> as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ComponentRecord {
    pub description: String,
    pub actions: Vec<RecordReference>,
}

impl Encode for ComponentRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.description.encode(&mut buf);
        self.actions.encode(&mut buf);
    }
}

impl Decode for ComponentRecord {
    type Error = ComponentRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let description = String::decode(&mut buf).map_err(ComponentRecordError::Description)?;
        let actions = Vec::decode(&mut buf).map_err(ComponentRecordError::Actions)?;

        Ok(Self {
            description,
            actions,
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
