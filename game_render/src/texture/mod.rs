pub mod image;

use std::collections::VecDeque;

use glam::UVec2;
use slotmap::{DefaultKey, SlotMap};

pub use self::image::{Image, ImageFormat, ImageId, Images, TextureFormat};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderImageId(DefaultKey);

pub struct RenderTextures {
    textures: SlotMap<DefaultKey, RenderTexture>,
    pub(crate) events: VecDeque<RenderTextureEvent>,
}

impl RenderTextures {
    pub fn new() -> Self {
        Self {
            textures: SlotMap::new(),
            events: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, texture: RenderTexture) -> RenderImageId {
        let key = self.textures.insert(texture);
        self.events
            .push_back(RenderTextureEvent::Create(RenderImageId(key), texture));
        RenderImageId(key)
    }

    pub fn remove(&mut self, id: RenderImageId) -> Option<RenderTexture> {
        let val = self.textures.remove(id.0)?;
        self.events.push_back(RenderTextureEvent::Destroy(id));
        Some(val)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RenderTexture {
    pub size: UVec2,
}

pub(crate) enum RenderTextureEvent {
    Create(RenderImageId, RenderTexture),
    Destroy(RenderImageId),
}
