use bytes::{Buf, BufMut};
use game_common::components::transform::Transform;
use glam::{Quat, Vec3};

use crate::material::Material;
use crate::textures::Texture;
use crate::vertex::Vertices;
use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct Mesh {
    pub transform: Transform,
    pub vertices: Vertices,
    pub material: Material,
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

impl Encode for Mesh {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.transform.encode(&mut buf);
        self.vertices.encode(&mut buf);
        self.material.encode(&mut buf);

        (self.textures.len() as u16).encode(&mut buf);
        for tex in &self.textures {
            tex.encode(&mut buf);
        }
    }
}

impl Decode for Mesh {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let transform = Transform::decode(&mut buf)?;
        let vertices = Vertices::decode(&mut buf)?;
        let material = Material::decode(&mut buf)?;

        let num_textures = u16::decode(&mut buf)?;
        let mut textures = Vec::new();
        for _ in 0..num_textures {
            let tex = Texture::decode(&mut buf)?;
            textures.push(tex);
        }

        Ok(Self {
            transform,
            vertices,
            material,
            textures,
        })
    }
}
