#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
mod x86;

use image::{GenericImage, GenericImageView, ImageBuffer, Rgba};

/// Overlays top at position (x,y).
///
/// # Safety
///  
/// top must fit on bottom without overlapping, i.e. `bottom.width() >= top.width() + x` and
/// `bottom.height() >= top.height() + y` must both be true.
pub unsafe fn overlay_unchecked(
    bottom: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    top: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: u32,
    y: u32,
) {
    if cfg!(debug_assertions) {
        assert!(bottom.width() >= top.width() + x);
        assert!(bottom.height() >= top.height() + y);
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "sse"
    ))]
    {
        unsafe { x86::overlay(bottom, top, x, y) };
        return;
    }

    // Naive impl
    // Note that this may be "unreachable" if a specialized impl
    // is chosen.
    #[allow(unreachable_code)]
    {
        image::imageops::overlay(bottom, top, x as i64, y as i64);
    }
}

/// Overlays the image `top` at position (`x`, `y`) of `bottom`, while dynamically loading every
/// pixel of `bottom` as `color`.
///
/// # Safeety
///
///
pub(super) fn overlay_background_color_unchecked(
    bottom: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    color: Rgba<u8>,
    top: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: u32,
    y: u32,
) {
    debug_assert!(bottom.width() >= top.width() + x);
    debug_assert!(bottom.height() >= top.height() + y);

    let start_x = x;
    let start_y = y;

    for x in 0..bottom.width() {
        for y in 0..bottom.height() {
            let top_x = x - start_x;
            let top_y = y - start_y;

            if top_x > top.width() {
                unsafe {
                    bottom.unsafe_put_pixel(x, y, color);
                }
            } else {
                unsafe {
                    let bottom_px = color;
                    let top_px = top.unsafe_get_pixel(top_x, top_y);
                    let pixel = blend(bottom_px, top_px);
                    bottom.unsafe_put_pixel(x, y, pixel);
                }
            }
        }
    }
}

unsafe fn blend(bottom: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "sse"
    ))]
    return unsafe { Rgba(x86::blend(bottom.0, top.0)) };

    #[allow(unreachable_code)]
    {
        let mut bottom = bottom;
        image::Pixel::blend(&mut bottom, &top);
        bottom
    }
}
