use std::convert::Infallible;

use game_tracing::trace_span;
use image::{ImageBuffer, Rgba};
use thiserror::Error;
use tiny_skia::{PixmapMut, BYTES_PER_PIXEL};
use usvg::{Options, Size, Transform, Tree};

use crate::reactive::Context;
use crate::style::Color;

use super::{Container, Image, Widget};

/// A SVG image widget.
///
/// # Limitations
///
/// `Svg` does not support the following SVG features:
/// - No animations.
/// - No text rendering.
#[derive(Debug)]
pub struct Svg {
    data: SvgData,
    width: u32,
    height: u32,
    style: SvgStyle,
}

impl Svg {
    /// Creates a new `Svg` element, rasterized with the given `width` and `height`.
    pub fn new(data: SvgData, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
            style: SvgStyle::default(),
        }
    }

    pub fn style(mut self, style: SvgStyle) -> Self {
        self.style = style;
        self
    }

    fn render_to_image(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let size = self.width as usize * self.height as usize * BYTES_PER_PIXEL;

        let mut buf = vec![0; size];
        let mut pixmap = PixmapMut::from_bytes(&mut buf, self.width, self.height).unwrap();

        // SVGs can come with their own viewBox element which causes them
        // to be rendered at the size they requested.
        // To always convert them into `self.width,self.height` we scale
        // their viewport down to the size we expect.
        // To prevent stretching amoung any axis we should always take
        // a uniform value for both x and y scale.
        let viewport_size = self.data.0.size();
        let scale_x = self.width as f32 / viewport_size.width();
        let scale_y = self.height as f32 / viewport_size.height();
        let scale = f32::min(scale_x, scale_y);
        debug_assert!(scale_x.is_sign_positive());
        debug_assert!(scale_y.is_sign_positive());

        // We may have scaled one direction too much to fit into `self.width,self.height`.
        // To "fix" this we translate the SVG root to the center of our viewport.
        // Note that we only translate one direction.
        let translate_x = ((scale_x / scale) - 1.0) * scale_x * self.width as f32;
        let translate_y = ((scale_y / scale) - 1.0) * scale_y * self.height as f32;
        debug_assert!(translate_x.is_sign_positive());
        debug_assert!(translate_y.is_sign_positive());

        let transform =
            Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);

        resvg::render(&self.data.0, transform, &mut pixmap);

        let mut img =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(self.width, self.height, buf).unwrap();

        if let Some(color) = self.style.color {
            for px in img.pixels_mut() {
                let a = px.0[3] as f32 / 255.0;
                let r = color.0 .0[0] as f32 / 255.0;
                let g = color.0 .0[1] as f32 / 255.0;
                let b = color.0 .0[2] as f32 / 255.0;

                px.0[0] = (r * a * 255.0) as u8;
                px.0[1] = (g * a * 255.0) as u8;
                px.0[2] = (b * a * 255.0) as u8;
            }
        }

        img
    }
}

impl Widget for Svg {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let _span = trace_span!("Svg::mount").entered();

        let wrapper = Container::new().mount(parent);

        if self.width != 0 && self.height != 0 {
            Image::new().image(self.render_to_image()).mount(&wrapper);
        }

        wrapper
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SvgError(usvg::Error);

#[derive(Clone, Debug)]
pub struct SvgData(Tree);

impl SvgData {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SvgError> {
        let mut options = Options::default();
        options.default_size = Size::from_wh(1.0, 1.0).unwrap();

        Tree::from_data(bytes, &options).map(Self).map_err(SvgError)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SvgStyle {
    /// Overwrites the color of all non-transparent SVG elements.
    pub color: Option<Color>,
}

impl crate::runtime_v2::Widget for Svg {
    type Message = Infallible;

    fn render(&self, _ctx: &crate::runtime_v2::Context<Self>) -> crate::runtime_v2::View {
        Image::new().image(self.render_to_image()).into()
    }
}
