use glam::UVec2;
use image::ImageBuffer;

use super::computed_style::{ComputedBounds, ComputedStyle};
use super::{BuildPrimitiveElement, Image};

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
        let width = layout.max.x - layout.min.x;
        let height = layout.max.y - layout.min.y;

        // `Image` will already render a debugging border around
        // the container.
        let image = ImageBuffer::new(width as u32, height as u32);
        Image { image }.build(style, layout, pipeline, device, queue, size)
    }

    fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        // FIXME: This is actually computed in LayoutTree, but this
        // is not good.
        unreachable!();

        let width = style.padding.left + style.padding.right;
        let height = style.padding.top + style.padding.bottom;

        ComputedBounds {
            min: UVec2::new(width, height),
            ..Default::default()
        }
    }
}
