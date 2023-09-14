use glam::UVec2;
use image::ImageBuffer;

use super::debug::is_debug_render_enabled;
use super::{BuildPrimitiveElement, Image};
use crate::layout::computed_style::ComputedStyle;

pub struct Container;

impl BuildPrimitiveElement for Container {
    fn build(
        &self,
        style: &ComputedStyle,
        layout: super::Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: UVec2,
    ) -> Option<super::PrimitiveElement> {
        // Truncate the container at the viewport size. This prevents rendering
        // potentially massive textures that destroy performance.
        let width = u32::min(layout.max.x - layout.min.x, size.x);
        let height = u32::min(layout.max.y - layout.min.y, size.y);

        if !style.style.background.is_none() || is_debug_render_enabled() {
            // `Image` will already render a debugging border around
            // the container.
            let image = ImageBuffer::new(width as u32, height as u32);
            Image { image }.build(style, layout, pipeline, device, queue, size)
        } else {
            None
        }
    }
}
