//! Types and (de)serializiers for data files.

pub mod header;
pub mod loader;
pub mod record;
pub mod uri;
pub mod varint;

use std::error::Error as StdError;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;
use std::string::FromUtf8Error;

use bytes::{Buf, BufMut};
use game_common::module::Module;
use header::{Header, HeaderError};
use record::{Record, RecordError};
use thiserror::Error;
use varint::VarU64;

pub trait Encode {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut;
}

pub trait Decode: Sized {
    type Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    EofError(#[from] EofError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error(transparent)]
    Header(#[from] HeaderError),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("unexpected eof reading {on}: consumed {consumed} by exepected {expected} bytes")]
pub struct EofError {
    pub on: &'static str,
    pub consumed: usize,
    pub expected: usize,
}

macro_rules! int_impls {
    ($($id:ident),*$(,)?) => {
        $(
            impl Encode for $id {
                #[inline]
                fn encode<B>(&self, buf: B)
                    where B: BufMut,
                {
                    self.to_le_bytes().encode(buf);
                }
            }

            impl Decode for $id {
                type Error = EofError;

                #[inline]
                fn decode<B>(buf: B) -> Result<Self, Self::Error>
                    where B: Buf,
                {
                    let bytes = <[u8; std::mem::size_of::<Self>()]>::decode(buf).map_err(|err| EofError {
                        on: stringify!($id),
                        consumed: err.consumed,
                        expected: err.expected,
                    })?;

                    Ok(Self::from_le_bytes(bytes))
                }
            }
        )*
    };
}

int_impls! {
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
    f32,
    f64,
}

impl Encode for str {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let len: VarU64 = self.len().into();
        len.encode(&mut buf);
        buf.put_slice(self.as_bytes());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StringError {
    #[error("failed to decode string bytes: {0}")]
    Bytes(ListError<u8>),
    #[error("invalid utf8: {0}")]
    InvalidUtf8(FromUtf8Error),
}

impl Decode for String {
    type Error = StringError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let bytes = Vec::decode(buf).map_err(StringError::Bytes)?;
        Self::from_utf8(bytes).map_err(StringError::InvalidUtf8)
    }
}

impl<const N: usize> Encode for [u8; N] {
    #[inline]
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(self);
    }
}

impl<const N: usize> Decode for [u8; N] {
    type Error = EofError;

    #[inline]
    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        // SAFETY: An uninitialized `[MaybeUninit<u8>; N]` is always valid.
        let mut bytes: [MaybeUninit<u8>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        let mut cursor = 0;

        while buf.remaining() > 0 && cursor < N {
            // SAFETY: `[MaybeUninit<u8>]` and `[u8]` have the same layout.
            let chunk: &[MaybeUninit<u8>] =
                unsafe { std::mem::transmute::<&[u8], &[MaybeUninit<u8>]>(buf.chunk()) };

            let count = std::cmp::min(chunk.len(), N - cursor);

            // SAFETY: Copy at most `start - N` bytes which never overflows `bytes` of size `N`.
            unsafe {
                let src = chunk.as_ptr();
                let dst = bytes.as_mut_ptr().add(cursor);

                std::ptr::copy_nonoverlapping(src, dst, count);
            }

            buf.advance(count);
            cursor += count;
        }

        if cursor != N {
            Err(EofError {
                on: "[u8; N]",
                consumed: cursor,
                expected: N,
            })
        } else {
            Ok(unsafe { std::ptr::read(bytes.as_ptr() as *const [u8; N]) })
        }
    }
}

impl<T> Encode for Vec<T>
where
    T: Encode,
{
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let len: VarU64 = self.len().into();
        len.encode(&mut buf);

        for elem in self {
            elem.encode(&mut buf);
        }
    }
}

#[derive(Debug, Error)]
pub enum ListError<T>
where
    T: Decode,
    <T as Decode>::Error: StdError,
{
    #[error("failed to decode list length: {0}")]
    Length(<VarU64 as Decode>::Error),
    #[error("failed to decode list element: {0}")]
    Element(<T as Decode>::Error),
}

impl<T> Clone for ListError<T>
where
    T: Decode,
    <T as Decode>::Error: StdError + Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Length(err) => Self::Length(*err),
            Self::Element(err) => Self::Element(err.clone()),
        }
    }
}

impl<T> Copy for ListError<T>
where
    T: Decode,
    <T as Decode>::Error: StdError + Copy,
{
}

impl<T> PartialEq for ListError<T>
where
    T: Decode,
    <T as Decode>::Error: StdError + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Length(lhs), Self::Length(rhs)) => lhs == rhs,
            (Self::Element(lhs), Self::Element(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl<T> Eq for ListError<T>
where
    T: Decode,
    <T as Decode>::Error: StdError + Eq,
{
}

impl<T> Hash for ListError<T>
where
    T: Decode,
    <T as Decode>::Error: StdError + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        let self_tag = std::mem::discriminant(self);
        self_tag.hash(state);

        match self {
            Self::Length(err) => {
                err.hash(state);
            }
            Self::Element(err) => {
                err.hash(state);
            }
        }
    }
}

impl<T> Decode for Vec<T>
where
    T: Decode,
    <T as Decode>::Error: StdError,
{
    type Error = ListError<T>;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len = VarU64::decode(&mut buf).map_err(ListError::Length)?;

        let mut list = Vec::new();

        for _ in 0..len.into() {
            let elem = T::decode(&mut buf).map_err(ListError::Element)?;
            list.push(elem);
        }

        Ok(list)
    }
}

#[derive(Clone, Debug)]
pub struct DataBuffer {
    pub header: Header,
    pub records: Vec<Record>,
}

impl DataBuffer {
    pub fn new(module: Module) -> Self {
        Self {
            header: Header { version: 0, module },
            records: Vec::new(),
        }
    }
}

impl Encode for DataBuffer {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.header.encode(&mut buf);

        (self.records.len() as u32).encode(&mut buf);
        for item in &self.records {
            item.encode(&mut buf);
        }
    }
}

impl Decode for DataBuffer {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let header = Header::decode(&mut buf)?;

        let num_records = u32::decode(&mut buf)?;
        let mut records = Vec::new();
        for _ in 0..num_records {
            let record = Record::decode(&mut buf)?;
            records.push(record);
        }

        Ok(Self { header, records })
    }
}

#[cfg(test)]
mod tests {

    use super::{Decode, Encode};

    #[test]
    fn test_array_decode() {
        let buf = [0, 1, 2, 3, 4];
        assert_eq!(<[u8; 5]>::decode(&buf[..]).unwrap(), [0, 1, 2, 3, 4]);

        let buf = [0, 1, 2, 3, 4, 5, 6, 7];
        assert_eq!(<[u8; 5]>::decode(&buf[..]).unwrap(), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_array_decode_fail_too_small() {
        let buf = [0, 1, 2];
        let err = <[u8; 5]>::decode(&buf[..]).unwrap_err();

        assert_eq!(err.consumed, 3);
        assert_eq!(err.expected, 5);
    }

    #[test]
    fn test_array_reflexive() {
        const LEN: usize = 8;
        let arr: [u8; LEN] = [54, 65, 97, 246, 97, 0, 56, 183];

        let mut buf = Vec::new();
        arr.encode(&mut buf);

        assert_eq!(arr, <[u8; LEN]>::decode(&buf[..]).unwrap());
    }

    #[test]
    fn test_int_decode() {
        let buf = 1234u32.to_le_bytes();
        assert_eq!(u32::decode(&buf[..]).unwrap(), 1234);

        let buf = [0; 5];
        assert_eq!(u32::decode(&buf[..]).unwrap(), 0);

        let buf = [0; 0];
        u32::decode(&buf[..]).unwrap_err();

        let buf = [0; 2];
        u32::decode(&buf[..]).unwrap_err();
    }

    #[test]
    fn test_string_encode() {
        let input = "Hello World";
        let output = [
            "Hello World".len().try_into().unwrap(),
            b'H',
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'W',
            b'o',
            b'r',
            b'l',
            b'd',
        ];

        let mut buf = Vec::new();
        input.encode(&mut buf);

        assert_eq!(buf, output);
    }
}
