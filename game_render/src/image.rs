use std::ops::{Deref, DerefMut};

use glam::Vec2;
use image::{ImageBuffer, Pixel, Rgba};

use crate::ui::{BuildPrimitiveElement, PrimitiveElement};

#[derive(Clone, Debug)]
pub struct Image {
    pub position: Vec2,
    pub dimensions: Vec2,
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl Image {
    pub fn dimensions(&self) -> Vec2 {
        self.dimensions
    }
}

impl BuildPrimitiveElement for Image {
    fn build(
        &self,
        pipeline: &crate::ui::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: glam::Vec2,
    ) -> PrimitiveElement {
        let start = crate::layout::remap(self.position, size);
        let end = crate::layout::remap(
            self.position + Vec2::new(self.image.width() as f32, self.image.height() as f32),
            size,
        );

        let mut img = self.image.clone();
        debug_border(&mut img);

        PrimitiveElement::new(
            pipeline,
            device,
            queue,
            start,
            end,
            &img,
            [1.0, 1.0, 1.0, 1.0],
        )
    }
}

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
