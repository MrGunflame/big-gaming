use game_asset::{Asset, LoadAsset};
use image::load_from_memory;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
}

impl Asset for Image {}

impl LoadAsset for Image {
    type Error = Box<dyn std::error::Error>;

    fn load(bytes: &[u8]) -> Result<Self, Self::Error> {
        let img = load_from_memory(bytes)?;
        let data = img.to_rgb8();

        Ok(Self {
            width: data.width(),
            height: data.height(),
            bytes: data.into_raw(),
            format: TextureFormat::Rgba8Unorm,
        })
    }
}
