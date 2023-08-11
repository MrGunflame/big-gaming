use glam::{Vec2, Vec3, Vec4};

use crate::{Decode, Encode};

#[derive(Clone, Debug, Default)]
pub struct Vertices {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub tangents: Vec<Vec4>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u32>,
}

impl Encode for Vertices {
    fn encode<B>(&self, mut buf: B)
    where
        B: bytes::BufMut,
    {
        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.uvs.len());
        assert_eq!(self.positions.len(), self.tangents.len());

        (self.positions.len() as u32).encode(&mut buf);

        for position in &self.positions {
            position.encode(&mut buf);
        }

        for normal in &self.normals {
            normal.encode(&mut buf);
        }

        for tangent in &self.tangents {
            tangent.encode(&mut buf);
        }

        for uv in &self.uvs {
            uv.encode(&mut buf);
        }

        (self.indices.len() as u32).encode(&mut buf);
        for index in &self.indices {
            index.encode(&mut buf);
        }
    }
}

impl Decode for Vertices {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        let len = u32::decode(&mut buf)?;

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut tangents = Vec::new();

        for _ in 0..len {
            let position = Vec3::decode(&mut buf)?;
            positions.push(position);
        }

        for _ in 0..len {
            let normal = Vec3::decode(&mut buf)?;
            normals.push(normal);
        }

        for _ in 0..len {
            let tangent = Vec4::decode(&mut buf)?;
            tangents.push(tangent);
        }

        for _ in 0..len {
            let uv = Vec2::decode(&mut buf)?;
            uvs.push(uv);
        }

        let num_indices = u32::decode(&mut buf)?;
        let mut indices = Vec::new();
        for _ in 0..num_indices {
            let index = u32::decode(&mut buf)?;
            indices.push(index);
        }

        Ok(Self {
            positions,
            normals,
            uvs,
            tangents,
            indices,
        })
    }
}
