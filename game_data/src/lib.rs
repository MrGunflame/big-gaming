//! Types and (de)serializiers for data files.

use std::mem::MaybeUninit;

use bytes::{Buf, BufMut};
use game_common::module::Module;
use header::Header;
use record::{Record, RecordError};
use thiserror::Error;

pub mod components;
pub mod header;
pub mod loader;
pub mod record;

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
}

impl Encode for str {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        (self.len() as u64).encode(&mut buf);
        buf.put_slice(self.as_bytes());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StringError {
    #[error(transparent)]
    Eof(#[from] EofError),
}

impl Decode for String {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut len = u64::decode(&mut buf)? as usize;
        // 20MB max buffer to prevent memory exhaustion.
        let mut bytes = Vec::with_capacity(std::cmp::max(len, 20_000_000));

        while len > 0 {
            if buf.remaining() == 0 {
                return Err(EofError {
                    on: "String",
                    consumed: 0,
                    expected: 0,
                });
            }

            let chunk = buf.chunk();
            let end = std::cmp::min(chunk.len(), len);

            bytes.extend(&chunk[..end]);

            len -= end;
            buf.advance(end);
        }

        Ok(Self::from_utf8(bytes).unwrap())
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

#[derive(Clone, Debug)]
pub struct DataBuffer {
    pub header: Header,
    pub records: Vec<Record>,
}

impl DataBuffer {
    pub fn new(module: Module) -> Self {
        Self {
            header: Header {
                version: 0,
                items: 0,
                module,
            },
            records: Vec::new(),
        }
    }
}

impl Encode for DataBuffer {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        assert_eq!(self.header.items, self.records.len() as u32);

        self.header.encode(&mut buf);
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

        let mut records = vec![];
        for _ in 0..header.items {
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

        let buf = [0, 1, 2];
        <[u8; 5]>::decode(&buf[..]).unwrap_err();

        let buf = [0, 1, 2, 3, 4, 5, 6, 7];
        assert_eq!(<[u8; 5]>::decode(&buf[..]).unwrap(), [0, 1, 2, 3, 4]);
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
}
