use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ComponentRecordError {
    #[error("failed to decode component description: {0}")]
    Description(<String as Decode>::Error),
    #[error("failed to decode component script: {0}")]
    Script(<Uri as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ComponentRecord {
    pub description: String,
    pub script: Uri,
}

impl Encode for ComponentRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.description.encode(&mut buf);
        self.script.encode(&mut buf);
    }
}

impl Decode for ComponentRecord {
    type Error = ComponentRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let description = String::decode(&mut buf).map_err(ComponentRecordError::Description)?;
        let script = Uri::decode(&mut buf).map_err(ComponentRecordError::Script)?;

        Ok(Self {
            description,
            script,
        })
    }
}
