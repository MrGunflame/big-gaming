use game_render::texture::{Image, TextureFormat};
use image::ImageError;

pub fn parse_image(buf: &[u8]) -> Result<Image, ImageError> {
    let img = image::load_from_memory(buf)?.to_rgb8();

    Ok(Image {
        format: TextureFormat::Rgba8UnormSrgb,
        width: img.width(),
        height: img.height(),
        bytes: img.into_raw(),
    })
}
