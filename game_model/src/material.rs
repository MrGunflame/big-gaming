use bytes::{Buf, BufMut};

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaterialModel {
    /// The default metallic roughness model.
    #[default]
    MetallicRoughness,
}

impl Encode for MaterialModel {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let b: u8 = match self {
            Self::MetallicRoughness => 1,
        };

        b.encode(buf);
    }
}

impl Decode for MaterialModel {
    type Error = ();

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let b = u8::decode(buf)?;

        match b {
            1 => Ok(Self::MetallicRoughness),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MetallicRoughnessMaterial {
    /// RGBA base color
    pub base_color: [u8; 4],
    pub roughness: u8,
    pub metallic: u8,
    pub albedo_texture: u16,
    pub normal_texture: u16,
    pub metallic_roughness_texture: u16,
}

impl Encode for MetallicRoughnessMaterial {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.base_color.encode(&mut buf);
        self.roughness.encode(&mut buf);
        self.metallic.encode(&mut buf);
    }
}

impl Decode for MetallicRoughnessMaterial {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let base_color = <[u8; 4]>::decode(&mut buf)?;
        let roughness = u8::decode(&mut buf)?;
        let metallic = u8::decode(&mut buf)?;
        let albedo_texture = u16::decode(&mut buf)?;
        let normal_texture = u16::decode(&mut buf)?;
        let metallic_roughness_texture = u16::decode(&mut buf)?;

        Ok(Self {
            base_color,
            roughness,
            metallic,
            albedo_texture,
            normal_texture,
            metallic_roughness_texture,
        })
    }
}
