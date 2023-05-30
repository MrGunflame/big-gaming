use std::ops::{Deref, DerefMut};

use image::{ImageBuffer, Pixel, Rgba};

use super::computed_style::ComputedPadding;

/// Render a debugging border around the image.
pub fn debug_border<C>(image: &mut ImageBuffer<Rgba<u8>, C>)
where
    C: Deref<Target = [<Rgba<u8> as Pixel>::Subpixel]> + DerefMut,
{
    if image.width() == 0 || image.height() == 0 {
        return;
    }

    let pixel = Rgba([255, 0, 0, 255]);

    // Top border
    for index in 0..image.width() {
        image.put_pixel(index, 0, pixel);
    }

    // Left border
    for index in 0..image.height() {
        image.put_pixel(0, index, pixel);
    }

    // Botton border
    for index in 0..image.width() {
        image.put_pixel(index, image.height() - 1, pixel);
    }

    // Right border
    for index in 0..image.height() {
        image.put_pixel(image.width() - 1, index, pixel);
    }
}

const PADDING_COLOR: Rgba<u8> = Rgba([0x8b, 0x44, 0xf4, 255 / 2]);

pub fn debug_padding<C>(image: &mut ImageBuffer<Rgba<u8>, C>, padding: ComputedPadding)
where
    C: Deref<Target = [<Rgba<u8> as Pixel>::Subpixel]> + DerefMut,
{
    if cfg!(debug_assertions) {
        assert!(image.width() >= (padding.left + padding.right) as u32);
        assert!(image.height() >= (padding.top + padding.bottom) as u32);
    }

    // Top
    for y in 0..padding.top as u32 {
        for x in 0..image.width() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }

    // Bottom
    for y in image.height() - padding.bottom as u32..image.height() {
        for x in 0..image.width() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }

    // Left
    for x in 0..padding.left as u32 {
        for y in 0..image.height() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }

    // Right
    for x in image.width() - padding.right as u32..image.width() {
        for y in 0..image.height() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }
}
