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

#[derive(Clone, Debug)]
pub enum ComponentType {
    /// Unknown type or unspecified.
    Unknown,
    Empty,
    Map,
    Array,
    U8(Option<Clamp<u8>>),
    U16(Option<Clamp<u16>>),
    U32(Option<Clamp<u32>>),
    U64(Option<Clamp<u64>>),
    I8(Option<Clamp<i8>>),
    I16(Option<Clamp<i16>>),
    I32(Option<Clamp<i32>>),
    I64(Option<Clamp<i64>>),
    F32(Option<Clamp<f32>>),
    F64(Option<Clamp<f64>>),
}

const UNKNOWN: u8 = 0;
const EMPTY: u8 = 1;
const MAP: u8 = 2;
const ARRAY: u8 = 3;

const U8: u8 = 1 << 6;
const U16: u8 = (1 << 6) + 1;
const U32: u8 = (1 << 6) + 2;
const U64: u8 = (1 << 6) + 3;

const I8: u8 = (1 << 6) + 4;
const I16: u8 = (1 << 6) + 5;
const I32: u8 = (1 << 6) + 6;
const I64: u8 = (1 << 6) + 7;

const F32: u8 = (1 << 6) + 8;
const F64: u8 = (1 << 6) + 9;

impl Encode for ComponentType {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        match self {
            Self::Unknown => {
                UNKNOWN.encode(buf);
            }
            Self::Empty => {
                EMPTY.encode(buf);
            }
            Self::Map => {
                MAP.encode(buf);
            }
            Self::Array => {
                ARRAY.encode(buf);
            }
            Self::U8(clamp) => {
                let typ = U8 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::U16(clamp) => {
                let typ = U16 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::U32(clamp) => {
                let typ = U32 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::U64(clamp) => {
                let typ = U64 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::I8(clamp) => {
                let typ = I8 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::I16(clamp) => {
                let typ = I16 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::I32(clamp) => {
                let typ = I32 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::I64(clamp) => {
                let typ = I64 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::F32(clamp) => {
                let typ = F32 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
            Self::F64(clamp) => {
                let typ = F64 | (clamp.is_some() as u8) << 7;
                typ.encode(&mut buf);

                if let Some(clamp) = clamp {
                    clamp.encode(buf);
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Clamp<T> {
    min: T,
    max: T,
}

impl<T> Encode for Clamp<T>
where
    T: Encode,
{
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.min.encode(&mut buf);
        self.max.encode(&mut buf);
    }
}

impl<T> Decode for Clamp<T>
where
    T: Decode,
{
    type Error = <T as Decode>::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let min = T::decode(&mut buf)?;
        let max = T::decode(&mut buf)?;

        Ok(Self { min, max })
    }
}

// #[derive(Clone, Debug, Default)]
// pub struct Array {
//     pub elements: Vec<ComponentData>,
// }

// #[derive(Clone, Debug)]
// pub struct Map {
//     pub fields: Vec<(String, ComponentData)>,
// }
