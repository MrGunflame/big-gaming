use game_render::layout::remap;
use glam::Vec2;
use image::imageops::FilterType;
use image::{ImageBuffer, Rgba};

use super::computed_style::{ComputedBorderRadius, ComputedBounds, ComputedStyle};
use super::debug::{debug_border, debug_padding, is_debug_render_enabled};
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

        apply_border_radius(&mut img, style.border_radius);

        if is_debug_render_enabled() {
            debug_border(&mut img);
            debug_padding(&mut img, style.padding);
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

fn apply_border_radius(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    border_radius: ComputedBorderRadius,
) {
    if img.width() == 0 || img.height() == 0 {
        return;
    }

    let pixel = if is_debug_render_enabled() {
        Rgba([255, 255, 0, 255])
    } else {
        Rgba([0, 0, 0, 0])
    };

    // Top left
    let start = Vec2::new(0.0, 0.0);
    let end = Vec2::new(
        start.x + border_radius.top_left.ceil(),
        start.y + border_radius.top_left.ceil(),
    );

    for x in start.x as u32..end.x as u32 {
        for y in start.y as u32..end.y as u32 {
            let distance = (end - Vec2::new(x as f32, y as f32)).length();

            if distance > border_radius.top_left {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }

    // Bottom left
    let start = Vec2::new(0.0, img.height() as f32);
    let end = Vec2::new(
        start.x + border_radius.bottom_left.ceil(),
        start.y - border_radius.bottom_left.floor() - 1.0,
    );

    for x in start.x as u32..end.x as u32 {
        for y in end.y as u32..start.y as u32 {
            let distance = (end - Vec2::new(x as f32, y as f32)).length();

            if distance > border_radius.bottom_left {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }

    // Top right
    let start = Vec2::new(img.width() as f32, 0.0);
    let end = Vec2::new(
        start.x - border_radius.top_right.floor() - 1.0,
        start.y + border_radius.top_right.ceil(),
    );

    for x in end.x as u32..start.x as u32 {
        for y in start.y as u32..end.y as u32 {
            let distance = (end - Vec2::new(x as f32, y as f32 as f32)).length();

            if distance > border_radius.top_right {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }

    // Bottom right
    let start = Vec2::new(img.width() as f32, img.height() as f32);
    let end = Vec2::new(
        start.x - border_radius.bottom_right.floor() - 1.0,
        start.y - border_radius.bottom_right.floor() - 1.0,
    );

    for x in end.x as u32..start.x as u32 {
        for y in end.y as u32..start.y as u32 {
            let distance = (end - Vec2::new(x as f32, y as f32)).length();

            if distance > border_radius.bottom_right {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use image::ImageBuffer;

    use crate::render::computed_style::ComputedBorderRadius;

    use super::apply_border_radius;

    #[test]
    fn border_radius() {
        let mut image = ImageBuffer::new(100, 100);
        let border_radius = ComputedBorderRadius {
            top_left: 10.0,
            bottom_left: 10.0,
            top_right: 10.0,
            bottom_right: 10.0,
        };

        apply_border_radius(&mut image, border_radius);
    }

    #[test]
    fn border_radius_image_empty() {
        let mut image = ImageBuffer::new(0, 0);
        let border_radius = ComputedBorderRadius {
            top_left: 10.0,
            bottom_left: 10.0,
            top_right: 10.0,
            bottom_right: 10.0,
        };

        apply_border_radius(&mut image, border_radius);
    }

    #[test]
    fn border_radius_image_too_small() {
        let mut image = ImageBuffer::new(10, 10);
        let border_radius = ComputedBorderRadius {
            top_left: 20.0,
            bottom_left: 20.0,
            top_right: 20.0,
            bottom_right: 20.0,
        };

        apply_border_radius(&mut image, border_radius);
    }
}
