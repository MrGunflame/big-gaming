use bytes::{Buf, BufMut};
use game_common::components::transform::Transform;
use glam::{Quat, Vec3};

use crate::textures::Texture;
use crate::vertex::Vertices;
use crate::{Decode, Encode};

pub struct Mesh {
    pub transform: Transform,
    pub vertices: Vertices,
    pub textures: Vec<Texture>,
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
