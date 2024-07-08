use glam::UVec2;
use image::{ImageBuffer, Rgba};

use crate::layout::computed_style::{ComputedBounds, ComputedStyle};
use crate::render::Text;
use crate::style::Style;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Clone, Debug, Default)]
pub struct Primitive {
    pub style: Style,
    pub image: Option<Image>,
    pub text: Option<Text>,
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
    pub(crate) fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        let mut size = UVec2::new(
            style.padding.left + style.padding.right,
            style.padding.top + style.padding.bottom,
        );

        if let Some(image) = &self.image {
            size = size.saturating_add(UVec2::new(image.width(), image.height()));
        }

        if let Some(text) = &self.text {
            let img = crate::render::text::render_to_texture(
                &text.text,
                text.size,
                UVec2::ZERO,
                text.caret,
            );

            size = size.saturating_add(UVec2::new(img.width(), img.height()));
        }

        ComputedBounds {
            min: size,
            max: size,
        }
    }
}
