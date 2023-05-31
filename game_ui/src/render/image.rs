use game_render::layout::remap;
use glam::Vec2;
use image::imageops::FilterType;
use image::{ImageBuffer, Rgba};

use super::computed_style::{ComputedBounds, ComputedStyle};
use super::debug::{debug_border, debug_padding};
use super::style::Background;
use super::{BuildPrimitiveElement, PrimitiveElement, Rect};

#[derive(Clone, Debug)]
pub struct Image {
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl BuildPrimitiveElement for Image {
    fn build(
        &self,
        style: &ComputedStyle,
        position: Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: Vec2,
    ) -> Option<PrimitiveElement> {
        let min = remap(position.min, size);
        let max = remap(position.max, size);

        let padding = Vec2::new(
            style.padding.left + style.padding.right,
            style.padding.top + style.padding.bottom,
        );

        dbg!(padding);

        let width = ((position.max.x - position.min.x) as u32) + padding.x as u32;
        let height = ((position.max.y - position.min.y) as u32) + padding.y as u32;

        let mut img = match &style.style.background {
            Background::None => {
                let mut img = ImageBuffer::new(width, height);
                image::imageops::overlay(
                    &mut img,
                    &self.image,
                    style.padding.left as i64,
                    style.padding.top as i64,
                );

                img
            }
            Background::Color(color) => {
                let mut img = ImageBuffer::from_fn(width, height, |_, _| *color);
                image::imageops::overlay(
                    &mut img,
                    &self.image,
                    style.padding.left as i64,
                    style.padding.top as i64,
                );
                img
            }
            Background::Image(image) => {
                let mut img = image::imageops::resize(image, width, height, FilterType::Nearest);
                image::imageops::overlay(
                    &mut img,
                    &self.image,
                    style.padding.left as i64,
                    style.padding.top as i64,
                );
                img
            }
        };

        if cfg!(feature = "debug_render") {
            if std::env::var("UI_DEBUG_RENDER").is_ok() {
                debug_border(&mut img);
                debug_padding(&mut img, style.padding);
            }
        }

        Some(PrimitiveElement::new(
            pipeline,
            device,
            queue,
            Rect { min, max },
            &img,
            style.style.color.to_f32(),
        ))
    }

    fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        let width = style.padding.left + style.padding.right;
        let height = style.padding.top + style.padding.bottom;

        let size = Vec2::new(
            self.image.width() as f32 + width,
            self.image.height() as f32 + height,
        );

        ComputedBounds {
            min: size,
            max: size,
        }
    }
}
