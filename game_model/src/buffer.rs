use bytes::{Buf, BufMut};
use glam::{Vec2, Vec3, Vec4};

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct Buffer {
    pub bytes: Vec<u8>,
}

impl Buffer {
    pub fn as_positions(&self) -> &[Vec3] {
        bytemuck::cast_slice(&self.bytes)
    }

    pub fn as_normals(&self) -> &[Vec3] {
        bytemuck::cast_slice(&self.bytes)
    }

    pub fn as_tangents(&self) -> &[Vec4] {
        bytemuck::cast_slice(&self.bytes)
    }

    pub fn as_uvs(&self) -> &[Vec2] {
        bytemuck::cast_slice(&self.bytes)
    }

    pub fn as_indices(&self) -> &[u32] {
        bytemuck::cast_slice(&self.bytes)
    }
}

impl Encode for Buffer {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        (self.bytes.len() as u32).encode(&mut buf);
        buf.put_slice(&self.bytes);
    }
}

impl Decode for Buffer {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len = u32::decode(&mut buf)?;
        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(u8::decode(&mut buf)?);
        }

        Ok(Self { bytes })
    }
}
