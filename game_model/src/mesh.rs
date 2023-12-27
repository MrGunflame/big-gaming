use bytes::{Buf, BufMut};
use game_common::components::Transform;
use glam::{Quat, Vec3};

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions: u16,
    pub normals: u16,
    pub tangents: u16,
    pub uvs: u16,
    pub indices: u16,
}

impl Encode for Transform {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.translation.encode(&mut buf);
        self.rotation.encode(&mut buf);
        self.scale.encode(&mut buf);
    }
}

impl Decode for Transform {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let translation = Vec3::decode(&mut buf)?;
        let rotation = Quat::decode(&mut buf)?;
        let scale = Vec3::decode(&mut buf)?;

        Ok(Self {
            translation,
            rotation,
            scale,
        })
    }
}

impl Encode for Mesh {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.positions.encode(&mut buf);
        self.normals.encode(&mut buf);
        self.tangents.encode(&mut buf);
        self.uvs.encode(&mut buf);
        self.indices.encode(&mut buf);
    }
}

impl Decode for Mesh {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let positions = u16::decode(&mut buf)?;
        let normals = u16::decode(&mut buf)?;
        let tangents = u16::decode(&mut buf)?;
        let uvs = u16::decode(&mut buf)?;
        let indices = u16::decode(&mut buf)?;

        Ok(Self {
            positions,
            normals,
            tangents,
            uvs,
            indices,
        })
    }
}
