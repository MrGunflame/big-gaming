use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode};

use super::item::ItemComponent;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ObjectRecordError {
    #[error("failed to decode uri: {0}")]
    Uri(<Uri as Decode>::Error),
    #[error("failed to decode components: {0}")]
    Components(<Vec<ItemComponent> as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ObjectRecord {
    pub uri: Uri,
    pub components: Vec<ItemComponent>,
}

impl Encode for ObjectRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.uri.encode(&mut buf);
        self.components.encode(&mut buf);
    }
}

impl Decode for ObjectRecord {
    type Error = ObjectRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let uri = Uri::decode(&mut buf).map_err(ObjectRecordError::Uri)?;
        let components = Vec::decode(&mut buf).map_err(ObjectRecordError::Components)?;

        Ok(Self { uri, components })
    }
}
