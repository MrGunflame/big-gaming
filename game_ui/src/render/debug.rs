use std::ops::{Deref, DerefMut};

use image::{ImageBuffer, Pixel, Rgba};

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
