use bytes::{Buf, BufMut};
use indexmap::IndexMap;
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
    Map(Map),
    Array(Box<ComponentType>),
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
            Self::Map(map) => {
                MAP.encode(&mut buf);
                map.encode(buf);
            }
            Self::Array(elems) => {
                ARRAY.encode(&mut buf);
                elems.encode(buf);
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ComponentTypeError {
    #[error("failed to decode component type byte: {0}")]
    Byte(<u8 as Decode>::Error),
    #[error("invalid component type: {0}")]
    InvalidKind(u8),
    #[error("failed to decode map: {0}")]
    Map(Box<<Map as Decode>::Error>),
    #[error("failed to decode array: {0}")]
    Array(Box<ComponentTypeError>),
    #[error(transparent)]
    U8(<Clamp<u8> as Decode>::Error),
    #[error(transparent)]
    U16(<Clamp<u16> as Decode>::Error),
    #[error(transparent)]
    U32(<Clamp<u32> as Decode>::Error),
    #[error(transparent)]
    U64(<Clamp<u64> as Decode>::Error),
    #[error(transparent)]
    I8(<Clamp<i8> as Decode>::Error),
    #[error(transparent)]
    I16(<Clamp<i16> as Decode>::Error),
    #[error(transparent)]
    I32(<Clamp<i32> as Decode>::Error),
    #[error(transparent)]
    I64(<Clamp<i64> as Decode>::Error),
    #[error(transparent)]
    F32(<Clamp<f32> as Decode>::Error),
    #[error(transparent)]
    F64(<Clamp<f64> as Decode>::Error),
}

impl Decode for ComponentType {
    type Error = ComponentTypeError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let typ = u8::decode(&mut buf).map_err(ComponentTypeError::Byte)?;

        // Extract the clamp marker bit.
        let clamp = typ & 1 << 7 == 1;
        let typ = typ & !(1 << 7);

        match typ {
            UNKNOWN => Ok(Self::Unknown),
            EMPTY => Ok(Self::Empty),
            MAP => {
                let map = Map::decode(buf).map_err(|e| ComponentTypeError::Map(Box::new(e)))?;
                Ok(Self::Map(map))
            }
            ARRAY => {
                let elems = ComponentType::decode(&mut buf)
                    .map_err(|e| ComponentTypeError::Array(Box::new(e)))?;
                Ok(Self::Array(Box::new(elems)))
            }
            U8 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::U8)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::U8(clamp))
            }
            U16 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::U16)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::U16(clamp))
            }
            U32 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::U32)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::U32(clamp))
            }
            U64 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::U64)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::U64(clamp))
            }
            I8 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::I8)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::I8(clamp))
            }
            I16 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::I16)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::I16(clamp))
            }
            I32 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::I32)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::I32(clamp))
            }
            I64 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::I64)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::I64(clamp))
            }
            F32 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::F32)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::F32(clamp))
            }
            F64 => {
                let clamp = if clamp {
                    let c = Clamp::decode(&mut buf).map_err(ComponentTypeError::F64)?;
                    Some(c)
                } else {
                    None
                };

                Ok(Self::F64(clamp))
            }
            _ => Err(ComponentTypeError::InvalidKind(typ)),
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

#[derive(Clone, Debug, Default)]
pub struct Map {
    // Note that the order is important.
    fields: IndexMap<String, ComponentType>,
}

impl Map {
    pub fn new() -> Self {
        Self {
            fields: IndexMap::new(),
        }
    }

    pub fn push(&mut self, key: String, typ: ComponentType) {
        self.fields.insert(key, typ);
    }

    pub fn get(&self, key: &str) -> Option<&ComponentType> {
        self.fields.get(key)
    }
}

impl Encode for Map {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        (self.fields.len() as u64).encode(&mut buf);

        for (key, typ) in self.fields.iter() {
            key.encode(&mut buf);
            typ.encode(&mut buf);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum MapError {
    #[error("failed to decode map length: {0}")]
    Length(<u64 as Decode>::Error),
    #[error("failed to decode map key: {0}")]
    Key(<String as Decode>::Error),
    #[error("failed to decode map type: {0}")]
    Type(<ComponentType as Decode>::Error),
}

impl Decode for Map {
    type Error = MapError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len = u64::decode(&mut buf).map_err(MapError::Length)?;

        let mut fields = IndexMap::new();
        for _ in 0..len {
            let key = String::decode(&mut buf).map_err(MapError::Key)?;
            let typ = ComponentType::decode(&mut buf).map_err(MapError::Type)?;

            fields.insert(key, typ);
        }

        Ok(Self { fields })
    }
}
