use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ActionRecordError {
    #[error("failed to decode action description: {0}")]
    Description(<String as Decode>::Error),
    #[error("failed to decode action script uri: {0}")]
    Script(<Uri as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ActionRecord {
    pub description: String,
}

impl Encode for ActionRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.description.encode(&mut buf);
    }
}

impl Decode for ActionRecord {
    type Error = ActionRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let description = String::decode(&mut buf).map_err(ActionRecordError::Description)?;

        Ok(Self { description })
    }
}
