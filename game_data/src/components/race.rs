use bytes::{Buf, BufMut};
use game_common::record::RecordReference;
use thiserror::Error;

use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RaceRecordError {
    #[error("failed to decode race actions: {0}")]
    Actions(<Vec<RecordReference> as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct RaceRecord {
    pub actions: Vec<RecordReference>,
}

impl Encode for RaceRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.actions.encode(&mut buf);
    }
}

impl Decode for RaceRecord {
    type Error = RaceRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let actions = Vec::decode(&mut buf).map_err(RaceRecordError::Actions)?;

        Ok(Self { actions })
    }
}
