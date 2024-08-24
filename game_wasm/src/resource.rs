use crate::encoding::{Decode, DecodeError, Encode, Primitive, Reader, Writer};
use crate::world::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceId {
    Record(RecordResourceId),
    Runtime(RuntimeResourceId),
}

impl ResourceId {
    #[inline]
    pub const fn is_record(&self) -> bool {
        matches!(self, Self::Record(_))
    }

    #[inline]
    pub const fn is_runtime(&self) -> bool {
        matches!(self, Self::Runtime(_))
    }
}

impl Encode for ResourceId {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        match self {
            Self::Record(id) => {
                let bytes = id.0.into_bytes();
                writer.write(Primitive::Bytes, &bytes);
            }
            Self::Runtime(id) => {
                writer.write(Primitive::Bytes, &[0; 12]);
                writer.write(Primitive::RuntimeResourceId, &id.to_bits().to_le_bytes());
            }
        }
    }
}

impl Decode for ResourceId {
    type Error = DecodeError;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let bytes = <[u8; 20]>::decode(reader)?;

        if bytes[0..12].iter().all(|v| *v == 0) {
            let bits = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
            Ok(Self::Runtime(RuntimeResourceId::from_bits(bits)))
        } else {
            Ok(Self::Record(RecordResourceId(RecordReference::from_bytes(
                bytes,
            ))))
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeResourceId(u64);

impl RuntimeResourceId {
    #[inline]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

impl Encode for RuntimeResourceId {
    fn encode<W>(&self, writer: W)
    where
        W: Writer,
    {
        self.0.encode(writer);
    }
}

impl Decode for RuntimeResourceId {
    type Error = DecodeError;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        u64::decode(reader).map(Self::from_bits)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordResourceId(pub RecordReference);

impl Encode for RecordResourceId {
    fn encode<W>(&self, writer: W)
    where
        W: Writer,
    {
        self.0.encode(writer);
    }
}

impl Decode for RecordResourceId {
    type Error = DecodeError;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        RecordReference::decode(reader).map(Self)
    }
}
