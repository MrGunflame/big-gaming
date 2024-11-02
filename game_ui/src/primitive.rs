use game_common::components::Color;
use game_tracing::trace_span;
use glam::UVec2;
use image::{ImageBuffer, Rgba};

use crate::layout::computed_style::{ComputedBounds, ComputedStyle};
use crate::render::debug::{debug_border, debug_padding, is_debug_render_enabled};
use crate::render::image::{apply_background, apply_border, apply_border_radius};
use crate::render::{DrawCommand, Rect, Text};
use crate::style::Style;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Clone, Debug, Default)]
pub struct Primitive {
    pub style: Style,
    pub image: Option<Image>,
    pub text: Option<Text<'static>>,
}

impl Primitive {
    /// Creates a new `Primitive` only using the given `style`.
    #[inline]
    pub fn from_style(style: Style) -> Self {
        Self {
            style,
            image: None,
            text: None,
        }
    }
}

impl Primitive {
    pub(crate) fn bounds(&self, style: &ComputedStyle, scale_factor: f64) -> ComputedBounds {
        let mut size = UVec2::new(
            style.padding.left + style.padding.right,
            style.padding.top + style.padding.bottom,
        );

        if let Some(image) = &self.image {
            size = size.saturating_add(UVec2::new(image.width(), image.height()));
        }

        if let Some(text) = &self.text {
            let mut text = text.as_ref();
            text.size *= scale_factor as f32;
            let bounds = text.bounds(style.bounds.max - style.bounds.min);
            size = size.saturating_add(bounds);
        }

        ComputedBounds {
            min: size,
            max: size,
        }
    }

    pub(crate) fn draw(
        &self,
        style: &ComputedStyle,
        layout: Rect,
        size: UVec2,
        scale_factor: f64,
    ) -> Option<DrawCommand> {
        let _span = trace_span!("Primitive::draw").entered();

        let mut img = match (&self.text, &self.image) {
            (Some(text), None) => {
                let mut text = text.as_ref();
                text.size *= scale_factor as f32;
                text.render_to_texture(style.bounds.max - style.bounds.min)
            }
            (None, Some(image)) => image.clone(),
            (None, None) => {
                // Truncate the container at the viewport size. This prevents rendering
                // potentially massive textures that destroy performance.
                let width = u32::min(layout.max.x - layout.min.x, size.x);
                let height = u32::min(layout.max.y - layout.min.y, size.y);

                if !style.style.background.is_none()
                    || !style.style.border.is_zero()
                    || is_debug_render_enabled()
                {
                    // `Image` will already render a debugging border around
                    // the container.
                    ImageBuffer::new(width, height)
                } else {
                    return None;
                }
            }
            (Some(_), Some(_)) => todo!(),
        };

        apply_border(&mut img, style);
        apply_background(&mut img, style);
        apply_border_radius(&mut img, style.border_radius);

        if is_debug_render_enabled() {
            debug_border(&mut img);
            debug_padding(&mut img, style.padding);
        }

        Some(DrawCommand {
            position: layout,
            color: Color::from_rgba(style.style.color.to_f32()),
            image: img,
        })
    }
}
