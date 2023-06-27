use game_render::texture::{Image, TextureFormat};
use glam::UVec2;
use image::ImageError;

pub fn parse_image(buf: &[u8]) -> Result<Image, ImageError> {
    let img = image::load_from_memory(buf)?.to_rgb8();

    Ok(Image::new(
        UVec2::new(img.width(), img.height()),
        TextureFormat::Rgba8Unorm,
        img.into_raw(),
    ))
}
