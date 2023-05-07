use image::ImageBuffer;

use super::debug::debug_border;
use super::layout::Bounds;
use super::{BuildPrimitiveElement, Image};

pub struct Container;

impl BuildPrimitiveElement for Container {
    fn build(
        &self,
        layout: super::Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: glam::Vec2,
    ) -> Option<super::PrimitiveElement> {
        let width = layout.max.x - layout.min.x;
        let height = layout.max.y - layout.min.y;

        // `Image` will already render a debugging border around
        // the container.
        let image = ImageBuffer::new(width as u32, height as u32);
        Image { image }.build(layout, pipeline, device, queue, size)
    }

    fn bounds(&self) -> super::layout::Bounds {
        Bounds::default()
    }
}
