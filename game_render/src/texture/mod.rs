pub mod image;

use glam::UVec2;
use slotmap::{DefaultKey, SlotMap};

pub use self::image::{Image, ImageFormat, ImageId, Images, TextureFormat};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderImageId(DefaultKey);

pub struct RenderTextures {
    textures: SlotMap<DefaultKey, RenderTexture>,
}

impl RenderTextures {
    pub fn new() -> Self {
        Self {
            textures: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, texture: RenderTexture) -> RenderImageId {
        let key = self.textures.insert(texture);
        RenderImageId(key)
    }
}

#[derive(Debug)]
pub struct RenderTexture {
    pub size: UVec2,
}
