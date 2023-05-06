use ab_glyph::{point, Font, FontRef, Glyph, Point, PxScale, ScaleFont};
use glam::Vec2;
use image::{ImageBuffer, Rgba, RgbaImage};

use super::image::Image;
use super::layout::Bounds;
use super::BuildPrimitiveElement;

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
}

impl BuildPrimitiveElement for Text {
    fn build(
        &self,
        layout: super::Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: glam::Vec2,
    ) -> Option<super::PrimitiveElement> {
        let image = render_to_texture(&self.text, self.size, layout.max - layout.min);

        Image { image }.build(layout, pipeline, device, queue, size)
    }

    fn bounds(&self) -> Bounds {
        let image = render_to_texture(&self.text, self.size, Vec2::splat(0.0));
        Image { image }.bounds()
    }
}

fn render_to_texture(text: &str, size: f32, max: Vec2) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let font =
        FontRef::try_from_slice(include_bytes!("/usr/share/fonts/droid/DroidSans.ttf")).unwrap();

    let scaled_font = font.as_scaled(PxScale::from(size));

    let mut glyphs = Vec::new();
    let (num_lines, max_width) =
        layout_glyphs(scaled_font, point(0.0, 0.0), text, 1000.0, &mut glyphs);

    let height = scaled_font.height().ceil() as u32;

    let mut image = RgbaImage::new(max_width.ceil() as u32, num_lines * height);

    // for pixel in image.pixels_mut() {
    //     *pixel = Rgba([0, 0, 0, 255]);
    // }

    for glyph in glyphs {
        if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined_glyph.px_bounds();

            outlined_glyph.draw(|x, y, alpha| {
                let pixel = (alpha * 255.0) as u8;

                image.put_pixel(
                    bounds.min.x as u32 + x,
                    bounds.min.y as u32 + y,
                    // Luma([pixel]),
                    Rgba([pixel, 0, 0, pixel]),
                );
            });
        }
    }

    image
}

fn layout_glyphs<SF: ScaleFont<F>, F: Font>(
    font: SF,
    position: Point,
    text: &str,
    max_width: f32,
    target: &mut Vec<Glyph>,
) -> (u32, f32) {
    let mut num_lines = 1;

    // The width of the widest line.
    let mut width = 0.0;

    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());

    let mut last_glyph: Option<Glyph> = None;

    for ch in text.chars() {
        if ch.is_control() {
            if ch == '\n' {
                if caret.x > width {
                    width = caret.x;
                }

                caret = point(position.x, caret.y + v_advance);
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

        if !ch.is_whitespace() && caret.x > position.x + max_width {
            if caret.x > width {
                width = caret.x;
            }

            caret = point(caret.x, position.y);
            glyph.position = caret;
            last_glyph = None;
            num_lines += 1;
        }

        target.push(glyph);
    }

    if caret.x > width {
        width = caret.x;
    }

    (num_lines, width)
}
