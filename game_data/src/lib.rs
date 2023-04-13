//! Types and (de)serializiers for data files.

use bytes::{Buf, BufMut};
use game_common::module::Module;
use header::Header;
use record::Record;
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("eof error")]
pub struct EofError;

macro_rules! int_impls {
    ($($id:ident),*$(,)?) => {
        $(
            impl Encode for $id {
                #[inline]
                fn encode<B>(&self, mut buf: B)
                    where B: BufMut,
                {
                    buf.put_slice(&self.to_le_bytes());
                }
            }

            impl Decode for $id {
                type Error = EofError;

                #[inline]
                fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
                    where B: Buf,
                {
                    let mut bytes = [0; std::mem::size_of::<Self>()];

                    let mut start = std::mem::size_of::<Self>();
                    while buf.remaining() > 0 && start < std::mem::size_of::<Self>() {
                        let chunk = buf.chunk();
                        let len = std::cmp::min(chunk.len(), std::mem::size_of::<Self>() - start);

                        // SAFETY: Copy at most n bytes, which never exceeds the size_of::<Self>().
                        unsafe {
                            let dst = bytes.as_mut_ptr().add(start);
                            std::ptr::copy_nonoverlapping(chunk.as_ptr(), dst, len);
                        }

                        buf.advance(len);
                        start += len;
                    }

                    if start != std::mem::size_of::<Self>() {
                        Err(EofError)
                    } else {
                        Ok(Self::from_le_bytes(bytes))
                    }

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
                return Err(EofError);
            }

            let chunk = buf.chunk();
            bytes.extend(chunk);

            len -= std::cmp::min(chunk.len(), len);
        }

        Ok(Self::from_utf8(bytes).unwrap())
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
    type Error = EofError;

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
    use super::Decode;

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
