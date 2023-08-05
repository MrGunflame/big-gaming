//! Model format

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

pub mod compression;
pub mod material;
pub mod mesh;
pub mod textures;
pub mod vertex;

use bytes::{Buf, BufMut};
use compression::CompressionScheme;
use glam::{Quat, Vec2, Vec3, Vec4};
use mesh::Mesh;

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
    pub compression: CompressionScheme,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        MAGIC.encode(&mut buf);
        self.version.encode(&mut buf);
        self.compression.encode(&mut buf);
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
        let compression = CompressionScheme::decode(&mut buf)?;

        Ok(Self {
            version,
            compression,
        })
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

impl Encode for Vec4 {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.x.encode(&mut buf);
        self.y.encode(&mut buf);
        self.z.encode(&mut buf);
        self.w.encode(&mut buf);
    }
}

impl Decode for Vec4 {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = f32::decode(&mut buf)?;
        let y = f32::decode(&mut buf)?;
        let z = f32::decode(&mut buf)?;
        let w = f32::decode(&mut buf)?;

        Ok(Self::new(x, y, z, w))
    }
}

impl Encode for Quat {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        Vec4::from(*self).encode(buf);
    }
}

impl Decode for Quat {
    type Error = ();

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Vec4::decode(buf).map(Self::from_vec4)
    }
}

/// The entire model file.
#[derive(Clone, Debug)]
pub struct Model {
    pub header: Header,
    pub meshes: Vec<Mesh>,
}

impl Encode for Model {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.header.encode(&mut buf);
        (self.meshes.len() as u16).encode(&mut buf);

        for m in &self.meshes {
            m.encode(&mut buf);
        }
    }
}

impl Decode for Model {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let header = Header::decode(&mut buf)?;

        let num_meshes = u16::decode(&mut buf)?;
        let mut meshes = Vec::new();

        for _ in 0..num_meshes {
            meshes.push(Mesh::decode(&mut buf)?);
        }

        Ok(Self { header, meshes })
    }
}
