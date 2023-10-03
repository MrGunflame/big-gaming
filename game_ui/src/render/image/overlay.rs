#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
mod x86;

use image::{ImageBuffer, Rgba};

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
