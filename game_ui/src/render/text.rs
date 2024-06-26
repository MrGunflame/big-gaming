use std::sync::OnceLock;

use ab_glyph::{point, Font, FontRef, Glyph, PxScale, ScaleFont};
use game_common::collections::lru::LruCache;
use game_tracing::trace_span;
use glam::UVec2;
use image::{ImageBuffer, Rgba, RgbaImage};
use parking_lot::Mutex;

use super::image::Image;
use super::{DrawCommand, DrawElement};
use crate::layout::computed_style::{ComputedBounds, ComputedStyle};

const DEFAULT_FONT: &[u8] = include_bytes!("../../../assets/fonts/OpenSans/OpenSans-Regular.ttf");

const TEXT_CACHE_CAP: usize = 1024;

static TEXT_CACHE: OnceLock<Mutex<LruCache<String, ImageBuffer<Rgba<u8>, Vec<u8>>>>> =
    OnceLock::new();

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub size: f32,
}

impl Text {
    #[inline]
    pub fn new<T>(text: T, size: f32) -> Self
    where
        T: ToString,
    {
        Self {
            text: text.to_string(),
            size,
        }
    }

    pub(crate) fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        let image = render_to_texture(&self.text, self.size, UVec2::splat(0));
        Image { image }.bounds(style)
    }
}

impl DrawElement for Text {
    fn draw(&self, style: &ComputedStyle, layout: super::Rect, size: UVec2) -> Option<DrawCommand> {
        let image = render_to_texture(&self.text, self.size, layout.max - layout.min);

        Image { image }.draw(style, layout, size)
    }
}

fn render_to_texture(text: &str, size: f32, max: UVec2) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    if let Some(res) = TEXT_CACHE
        .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
        .lock()
        .get(text)
    {
        return res.clone();
    }

    let _span = trace_span!("text::render_to_texture").entered();

    let font = FontRef::try_from_slice(DEFAULT_FONT).unwrap();

    let scaled_font = font.as_scaled(PxScale::from(size));

    let mut glyphs = Vec::new();
    let (num_lines, max_width) = layout_glyphs(scaled_font, text, 1000.0, &mut glyphs);

    // Note that `height()` returns the biggest point that may be drawn to.
    // We still need an additional pixel above that.
    let height = scaled_font.height().ceil() as u32 + 1;

    let mut image = RgbaImage::new(max_width.ceil() as u32 + 1, num_lines * height);

    for glyph in glyphs {
        if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined_glyph.px_bounds();

            outlined_glyph.draw(|x, y, cov| {
                let pixel = (cov * 255.0) as u8;

                image.put_pixel(
                    bounds.min.x as u32 + x,
                    bounds.min.y as u32 + y,
                    Rgba([pixel, pixel, pixel, pixel]),
                );
            });
        }
    }

    TEXT_CACHE
        .get()
        .unwrap()
        .lock()
        .insert(text.to_owned(), image.clone());
    image
}

fn layout_glyphs<SF: ScaleFont<F>, F: Font>(
    font: SF,
    text: &str,
    max_width: f32,
    target: &mut Vec<Glyph>,
) -> (u32, f32) {
    let mut num_lines = 1;
    let mut max_line_width = 0.0;

    let v_advance = font.height() + font.line_gap();
    let mut caret = point(0.0, font.ascent());

    let mut last_glyph: Option<Glyph> = None;

    for ch in text.chars() {
        if ch.is_control() {
            if ch == '\n' {
                max_line_width = f32::max(max_line_width, caret.x);

                caret = point(0.0, caret.y + v_advance);
                num_lines += 1;
            }

            continue;
        }

        let mut glyph = font.scaled_glyph(ch);
        if let Some(prev) = last_glyph.take() {
            caret.x += font.kern(prev.id, glyph.id);
        }

        glyph.position = caret;
        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if !ch.is_whitespace() && caret.x > max_width {
            max_line_width = f32::max(max_line_width, caret.x);

            caret = point(0.0, caret.y + v_advance);

            glyph.position = caret;

            last_glyph = None;
            num_lines += 1;
        }

        target.push(glyph);
    }

    max_line_width = f32::max(max_line_width, caret.x);

    (num_lines, max_line_width)
}

#[cfg(test)]
mod tests {
    use ab_glyph::{Font, FontRef, PxScale};
    use glam::UVec2;

    use super::{layout_glyphs, render_to_texture, DEFAULT_FONT};

    fn test_font() -> FontRef<'static> {
        FontRef::try_from_slice(DEFAULT_FONT).unwrap()
    }

    #[test]
    fn render_to_texture_singleline() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let max = UVec2::splat(0);

        render_to_texture(text, size, max);
    }

    #[test]
    fn render_to_texture_newline() {
        let text = "abcdefghijklmnopqrstuvwxyz\nabcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let max = UVec2::splat(0);

        render_to_texture(text, size, max);
    }

    #[test]
    fn render_to_texture_overflow() {
        let text: String = (0..1000).map(|_| "a").collect();
        let size = 10.0;
        let max = UVec2::splat(0);

        render_to_texture(&text, size, max);
    }

    #[test]
    fn layout_glyphs_width_too_small() {
        let font = test_font();
        let font = font.as_scaled(PxScale::from(100.0));
        let text = "Hello";
        let max_width = 1.0;
        let mut target = Vec::new();

        layout_glyphs(font, text, max_width, &mut target);
    }
}
