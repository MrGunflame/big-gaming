use game_render::layout::remap;
use glam::Vec2;
use image::{ImageBuffer, Rgba};

use super::debug::debug_border;
use super::layout::Bounds;
use super::{BuildPrimitiveElement, PrimitiveElement, Rect};

#[derive(Clone, Debug)]
pub struct Image {
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl BuildPrimitiveElement for Image {
    fn build(
        &self,
        position: Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: Vec2,
    ) -> Option<PrimitiveElement> {
        let min = remap(position.min, size);
        let max = remap(position.max, size);

        let mut img = self.image.clone();
        debug_border(&mut img);

        Some(PrimitiveElement::new(
            pipeline,
            device,
            queue,
            Rect { min, max },
            &img,
            [1.0, 1.0, 1.0, 1.0],
        ))
    }

    fn bounds(&self) -> Bounds {
        let size = Vec2::new(self.image.width() as f32, self.image.height() as f32);

        Bounds {
            min: size,
            max: size,
        }
    }
}
