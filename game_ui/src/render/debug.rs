use std::ops::{Deref, DerefMut};

use game_tracing::trace_span;
use image::{ImageBuffer, Pixel, Rgba};

use crate::layout::computed_style::ComputedPadding;

#[inline]
pub fn is_debug_render_enabled() -> bool {
    // Disable debug renderer at compile time if the cfg is set.
    #[cfg(ui_debug_render_disable)]
    #[inline]
    fn inner() -> bool {
        false
    }

    #[cfg(not(ui_debug_render_disable))]
    fn inner() -> bool {
        use std::sync::OnceLock;

        static DEBUG_RENDER_ENABLED: OnceLock<bool> = OnceLock::new();

        *DEBUG_RENDER_ENABLED.get_or_init(|| {
            if let Ok(val) = std::env::var("UI_DEBUG_RENDER") {
                match val.as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => {
                        tracing::warn!("invalid value for UI_DEBUG_RENDER env variable");
                        false
                    }
                }
            } else {
                false
            }
        })
    }

    inner()
}

/// Render a debugging border around the image.
pub fn debug_border<C>(image: &mut ImageBuffer<Rgba<u8>, C>)
where
    C: Deref<Target = [<Rgba<u8> as Pixel>::Subpixel]> + DerefMut,
{
    assert!(is_debug_render_enabled());
    let _span = trace_span!("debug_border").entered();

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
    assert!(is_debug_render_enabled());
    let _span = trace_span!("debug_padding").entered();

    if cfg!(debug_assertions) {
        assert!(image.width() >= (padding.left + padding.right));
        assert!(image.height() >= (padding.top + padding.bottom));
    }

    // Top
    for y in 0..padding.top {
        for x in 0..image.width() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }

    // Bottom
    for y in image.height() - padding.bottom..image.height() {
        for x in 0..image.width() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }

    // Left
    for x in 0..padding.left {
        for y in 0..image.height() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }

    // Right
    for x in image.width() - padding.right..image.width() {
        for y in 0..image.height() {
            image.get_pixel_mut(x, y).blend(&PADDING_COLOR);
        }
    }
}
