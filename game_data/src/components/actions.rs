use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ActionRecordError {
    #[error("failed to decode action description: {0}")]
    Description(<String as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ActionRecord {
    pub description: String,
}

impl Encode for ActionRecord {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.description.encode(buf);
    }
}

impl Decode for ActionRecord {
    type Error = ActionRecordError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let description = String::decode(buf).map_err(ActionRecordError::Description)?;

        Ok(Self { description })
    }
}
