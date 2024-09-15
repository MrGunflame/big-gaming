use alloc::collections::VecDeque;
use core::mem::MaybeUninit;

use alloc::vec::Vec;

use crate::encoding::{
    BinaryReader, BinaryWriter, Decode, DecodeError, Encode, Primitive, Reader, Writer,
};
use crate::raw::{
    resource_create_runtime, resource_get_runtime, resource_len_runtime, resource_update_runtime,
    RESULT_NO_ENTITY, RESULT_OK,
};
use crate::record::Record;
use crate::world::RecordReference;
use crate::Error;

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

impl From<RecordResourceId> for ResourceId {
    #[inline]
    fn from(value: RecordResourceId) -> Self {
        Self::Record(value)
    }
}

impl From<RuntimeResourceId> for ResourceId {
    #[inline]
    fn from(value: RuntimeResourceId) -> Self {
        Self::Runtime(value)
    }
}

impl From<RecordReference> for ResourceId {
    fn from(value: RecordReference) -> Self {
        Self::Record(RecordResourceId(value))
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

/// Creates a new runtime resource.
pub fn create_resource<T>(resource: &T) -> RuntimeResourceId
where
    T: Encode,
{
    let (fields, bytes) = BinaryWriter::new().encoded(resource);

    let mut id = MaybeUninit::uninit();
    match unsafe { resource_create_runtime(bytes.as_ptr(), bytes.len(), id.as_mut_ptr()) } {
        RESULT_OK => (),
        _ => unreachable!(),
    }

    let id = unsafe { id.assume_init() };
    RuntimeResourceId::from_bits(id)
}

/// Returns the resource with the given `id`.
///
/// # Errors
///
/// Returns an [`Error`] in the following cases:
/// - The resource with the given `id` does not exist.
/// - Decoding of the resource into `T` has failed.
pub fn get_resource<T>(id: ResourceId) -> Result<T, Error>
where
    T: Decode,
{
    let bytes = match id {
        ResourceId::Record(id) => {
            let record = Record::get(id.0)?;
            record.into_bytes()
        }
        ResourceId::Runtime(id) => unsafe {
            let mut len = MaybeUninit::uninit();
            match resource_len_runtime(id.to_bits(), len.as_mut_ptr()) {
                RESULT_OK => (),
                RESULT_NO_ENTITY => {
                    return Err(Error(crate::ErrorImpl::NoResource(ResourceId::Runtime(id))))
                }
                _ => unreachable!(),
            }

            let len = len.assume_init();
            let mut bytes = Vec::with_capacity(len);

            match resource_get_runtime(id.to_bits(), bytes.as_mut_ptr()) {
                RESULT_OK => (),
                _ => unreachable!(),
            }

            bytes.set_len(len);
            bytes
        },
    };

    crate::error!("{:?}", &bytes);

    let reader = BinaryReader::new(bytes, VecDeque::default());
    T::decode(reader).map_err(|_| Error(crate::ErrorImpl::ComponentDecode))
}

pub fn update_resource<T>(id: RuntimeResourceId, resource: &T) -> Result<(), Error>
where
    T: Encode,
{
    let (fields, bytes) = BinaryWriter::new().encoded(resource);
    match unsafe { resource_update_runtime(id.to_bits(), bytes.as_ptr(), bytes.len()) } {
        RESULT_OK => Ok(()),
        RESULT_NO_ENTITY => Err(Error(crate::ErrorImpl::NoResource(id.into()))),
        _ => unreachable!(),
    }
}
