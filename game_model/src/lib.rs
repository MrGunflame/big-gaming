//! Model format

pub mod material;
pub mod textures;
pub mod vertex;

use bytes::{Buf, BufMut};
use glam::{Vec2, Vec3};

pub const MAGIC: [u8; 4] = [0, 0, 0, 0];

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

#[derive(Copy, Clone, Debug)]
pub struct Header {
    // MAGIC
    pub version: u32,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        MAGIC.encode(&mut buf);
        self.version.encode(&mut buf);
    }
}

impl Decode for Header {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let magic = <[u8; 4]>::decode(&mut buf)?;
        assert_eq!(magic, MAGIC);

        let version = u32::decode(&mut buf)?;

        Ok(Self { version })
    }
}

impl<const N: usize> Encode for [u8; N] {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(self);
    }
}

impl<const N: usize> Decode for [u8; N] {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut bytes = [0; N];
        buf.copy_to_slice(&mut bytes);
        Ok(bytes)
    }
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
                type Error = ();

                #[inline]
                fn decode<B>(buf: B) -> Result<Self, Self::Error>
                    where B: Buf,
                {
                    let bytes = <[u8; std::mem::size_of::<Self>()]>::decode(buf).unwrap();
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

impl Encode for Vec3 {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.x.encode(&mut buf);
        self.y.encode(&mut buf);
        self.z.encode(&mut buf);
    }
}

impl Decode for Vec3 {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = f32::decode(&mut buf)?;
        let y = f32::decode(&mut buf)?;
        let z = f32::decode(&mut buf)?;

        Ok(Self { x, y, z })
    }
}

impl Encode for Vec2 {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.x.encode(&mut buf);
        self.y.encode(&mut buf);
    }
}

impl Decode for Vec2 {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = f32::decode(&mut buf)?;
        let y = f32::decode(&mut buf)?;

        Ok(Self { x, y })
    }
}
