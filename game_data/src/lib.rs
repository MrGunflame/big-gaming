//! Types and (de)serializiers for data files.

use bytes::{Buf, BufMut};
use components::item::ItemRecord;
use game_common::module::ModuleId;
use header::Header;

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
                type Error = std::io::Error;

                #[inline]
                fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
                    where B: Buf,
                {
                    let mut bytes = [0; std::mem::size_of::<Self>()];
                    buf.copy_to_slice(&mut bytes);
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

impl Decode for String {
    type Error = std::io::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut len = u64::decode(&mut buf)? as usize;
        // 20MB max buffer to prevent memory exhaustion.
        let mut bytes = Vec::with_capacity(std::cmp::max(len, 20_000_000));

        while len > 0 {
            if buf.remaining() == 0 {
                panic!();
            }

            let chunk = buf.chunk();
            bytes.extend(chunk);

            len -= std::cmp::min(chunk.len(), len);
        }

        Ok(Self::from_utf8(bytes).unwrap())
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataBuffer {
    pub header: Header,
    pub items: Vec<ItemRecord>,
}

impl DataBuffer {
    pub fn new() -> Self {
        Self {
            header: Header {
                version: 0,
                items: 0,
                id: ModuleId::random(),
            },
            items: Vec::new(),
        }
    }
}

impl Encode for DataBuffer {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        assert_eq!(self.header.items, self.items.len() as u32);

        self.header.encode(&mut buf);
        for item in &self.items {
            item.encode(&mut buf);
        }
    }
}

impl Decode for DataBuffer {
    type Error = std::io::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let header = Header::decode(&mut buf)?;

        let mut items = vec![];
        for _ in 0..header.items {
            let item = ItemRecord::decode(&mut buf)?;
            items.push(item);
        }

        Ok(Self { header, items })
    }
}
