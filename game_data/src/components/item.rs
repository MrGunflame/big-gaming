use std::fmt::{self, Display, Formatter, LowerHex};

use bytes::{Buf, BufMut};
use game_common::record::RecordReference;
use game_common::units::Mass;
use thiserror::Error;

use crate::uri::Uri;
use crate::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ItemRecordError {
    #[error("failed to decode item mass: {0}")]
    Mass(<Mass as Decode>::Error),
    #[error("failed to decode item value: {0}")]
    Value(<u64 as Decode>::Error),
    #[error("failed to decode tem uri: {0}")]
    Uri(<Uri as Decode>::Error),
    #[error("failed to decode item components: {0}")]
    Components(<Vec<ItemComponent> as Decode>::Error),
    #[error("failed to decode item actions: {0}")]
    Actions(<Vec<RecordReference> as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ItemRecord {
    pub mass: Mass,
    pub value: u64,
    pub uri: Uri,
    pub components: Vec<ItemComponent>,
    pub actions: Vec<RecordReference>,
}

impl Encode for ItemRecord {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.mass.encode(&mut buf);
        self.value.encode(&mut buf);
        self.uri.encode(&mut buf);
        self.components.encode(&mut buf);
        self.actions.encode(&mut buf);
    }
}

impl Decode for ItemRecord {
    type Error = ItemRecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mass = Mass::decode(&mut buf).map_err(ItemRecordError::Mass)?;
        let value = u64::decode(&mut buf).map_err(ItemRecordError::Value)?;
        let uri = Uri::decode(&mut buf).map_err(ItemRecordError::Uri)?;
        let components = Vec::decode(&mut buf).map_err(ItemRecordError::Components)?;
        let actions = Vec::decode(&mut buf).map_err(ItemRecordError::Actions)?;

        Ok(Self {
            mass,
            value,
            uri,
            components,
            actions,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

impl Encode for ItemId {
    #[inline]
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for ItemId {
    type Error = <u32 as Decode>::Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

impl Display for ItemId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ItemComponentError {
    #[error("failed to decode component record: {0}")]
    Record(<RecordReference as Decode>::Error),
    #[error("failed to decode component value: {0}")]
    Value(<Vec<u8> as Decode>::Error),
}

// FIXME: This is no longer exclusive to Item. rename
#[derive(Clone, Debug)]
pub struct ItemComponent {
    pub record: RecordReference,
    pub value: Vec<u8>,
}

impl Encode for ItemComponent {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.record.encode(&mut buf);
        self.value.encode(&mut buf);
    }
}

impl Decode for ItemComponent {
    type Error = ItemComponentError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let record = RecordReference::decode(&mut buf).map_err(ItemComponentError::Record)?;
        let value = Vec::decode(&mut buf).map_err(ItemComponentError::Value)?;

        Ok(Self { record, value })
    }
}
