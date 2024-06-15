use crate::{Decode, Encode, StringError};
use bytes::{Buf, BufMut};
use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecordReferenceError {
    #[error("failed to decode module: {0}")]
    Module(<ModuleId as Decode>::Error),
    #[error("failed to decode record: {0}")]
    Record(<RecordId as Decode>::Error),
}

impl Encode for RecordId {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for RecordId {
    type Error = <u32 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

impl Encode for RecordReference {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.module.encode(&mut buf);
        self.record.encode(&mut buf);
    }
}

impl Decode for RecordReference {
    type Error = RecordReferenceError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let module = ModuleId::decode(&mut buf).map_err(RecordReferenceError::Module)?;
        let record = RecordId::decode(&mut buf).map_err(RecordReferenceError::Record)?;

        Ok(Self { module, record })
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub id: RecordId,
    pub kind: RecordKind,
    pub name: String,
    pub description: String,
    pub data: Vec<u8>,
}

impl Encode for Record {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        RecordHeader {
            id: self.id,
            len: self.data.len().try_into().unwrap(),
            kind: self.kind,
        }
        .encode(&mut buf);

        self.name.encode(&mut buf);
        self.description.encode(&mut buf);
        self.data.encode(&mut buf);
    }
}

impl Decode for Record {
    type Error = RecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let header = RecordHeader::decode(&mut buf).map_err(RecordError::Header)?;
        let name = String::decode(&mut buf).map_err(RecordError::Name)?;
        let description = String::decode(&mut buf).map_err(RecordError::Description)?;
        let data = Vec::decode(&mut buf).map_err(RecordError::Data)?;

        Ok(Self {
            id: header.id,
            kind: header.kind,
            name,
            description,
            data,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordKind(pub RecordReference);

impl RecordKind {
    pub const COMPONENT: Self = Self(RecordReference {
        module: ModuleId::CORE,
        record: RecordId(0x01),
    });
}

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("failed to decode header: {0}")]
    Header(RecordHeaderError),
    #[error("failed to decode name: {0}")]
    Name(StringError),
    #[error("failed to decode description: {0}")]
    Description(StringError),
    #[error("failed to read data: {0}")]
    Data(<Vec<u8> as Decode>::Error),
}

/// ```text
/// 0               1               2               3
/// 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | RecordId                      | Length                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | RecordKind ModuleId                                         |
/// |                                                             |
/// |                                                             |
/// |                                                             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | RecordKind RecordId           | Reserved                    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Clone, Debug)]
pub struct RecordHeader {
    pub id: RecordId,
    pub len: u32,
    pub kind: RecordKind,
}

impl Encode for RecordHeader {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.len.encode(&mut buf);
        self.kind.0.encode(&mut buf);
        0u32.encode(&mut buf);
    }
}

impl Decode for RecordHeader {
    type Error = RecordHeaderError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordId::decode(&mut buf).map_err(RecordHeaderError::Id)?;
        let len = u32::decode(&mut buf).map_err(RecordHeaderError::Len)?;
        let kind = RecordReference::decode(&mut buf).map_err(RecordHeaderError::Kind)?;
        let _resv = u32::decode(&mut buf).map_err(RecordHeaderError::Resv)?;

        Ok(Self {
            id,
            len,
            kind: RecordKind(kind),
        })
    }
}

#[derive(Debug, Error)]
pub enum RecordHeaderError {
    #[error("failed to decode id: {0}")]
    Id(<RecordId as Decode>::Error),
    #[error("failed to decode len: {0}")]
    Len(<u32 as Decode>::Error),
    #[error("failed to decode kind: {0}")]
    Kind(<RecordReference as Decode>::Error),
    #[error("failed to decode resv: {0}")]
    Resv(<u32 as Decode>::Error),
}
