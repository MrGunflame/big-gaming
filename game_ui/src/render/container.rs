use glam::UVec2;
use image::ImageBuffer;

use super::debug::is_debug_render_enabled;
use super::{DrawCommand, DrawElement, Image};
use crate::layout::computed_style::ComputedStyle;

pub struct Container;

impl DrawElement for Container {
    fn draw(&self, style: &ComputedStyle, layout: super::Rect, size: UVec2) -> Option<DrawCommand> {
        // Truncate the container at the viewport size. This prevents rendering
        // potentially massive textures that destroy performance.
        let width = u32::min(layout.max.x - layout.min.x, size.x);
        let height = u32::min(layout.max.y - layout.min.y, size.y);

        if !style.style.background.is_none() || is_debug_render_enabled() {
            // `Image` will already render a debugging border around
            // the container.
            let image = ImageBuffer::new(width, height);
            Image { image }.draw(style, layout, size)
        } else {
            None
        }
    }
}
