use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ObjectRecordError {
    #[error("failed to decode uri: {0}")]
    Uri(<Uri as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ObjectRecord {
    pub uri: Uri,
}

impl Encode for ObjectRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.uri.encode(&mut buf);
    }
}

impl Decode for ObjectRecord {
    type Error = ObjectRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let uri = Uri::decode(&mut buf).map_err(ObjectRecordError::Uri)?;

        Ok(Self { uri })
    }
}
