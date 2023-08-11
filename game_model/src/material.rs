use bytes::{Buf, BufMut};

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub enum Material {
    MetallicRoughness(MetallicRoughnessMaterial),
}

impl Material {
    pub const fn model(&self) -> MaterialModel {
        match self {
            Self::MetallicRoughness(_) => MaterialModel::MetallicRoughness,
        }
    }
}

impl Encode for Material {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.model().encode(&mut buf);

        match self {
            Self::MetallicRoughness(material) => material.encode(buf),
        }
    }
}

impl Decode for Material {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let model = MaterialModel::decode(&mut buf)?;

        Ok(match model {
            MaterialModel::MetallicRoughness => {
                let material = MetallicRoughnessMaterial::decode(&mut buf)?;
                Self::MetallicRoughness(material)
            }
        })
    }
}

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
    // Use `0xFFFF` as the `None` variant so we can still have
    // 0-indexed textures.
    pub albedo_texture: Option<u16>,
    pub normal_texture: Option<u16>,
    pub metallic_roughness_texture: Option<u16>,
}

impl Encode for MetallicRoughnessMaterial {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.base_color.encode(&mut buf);
        self.roughness.encode(&mut buf);
        self.metallic.encode(&mut buf);

        let albedo_texture = self.albedo_texture.unwrap_or(u16::MAX);
        let normal_texture = self.normal_texture.unwrap_or(u16::MAX);
        let metallic_roughness_texture = self.metallic_roughness_texture.unwrap_or(u16::MAX);

        albedo_texture.encode(&mut buf);
        normal_texture.encode(&mut buf);
        metallic_roughness_texture.encode(&mut buf);
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
            albedo_texture: if albedo_texture == u16::MAX {
                None
            } else {
                Some(albedo_texture)
            },
            normal_texture: if normal_texture == u16::MAX {
                None
            } else {
                Some(normal_texture)
            },
            metallic_roughness_texture: if metallic_roughness_texture == u16::MAX {
                None
            } else {
                Some(metallic_roughness_texture)
            },
        })
    }
}
