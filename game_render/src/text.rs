use ab_glyph::{
    point, Font, FontRef, Glyph, Outline, OutlinedGlyph, Point, PxScale, PxScaleFactor, Rect,
    ScaleFont,
};
use image::{ImageBuffer, Luma, LumaA};

use crate::layout::{DrawContext, Widget};

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
}

impl Widget for Text {
    fn draw(&self, ctx: &mut DrawContext) {
        let font = FontRef::try_from_slice(include_bytes!("/usr/share/fonts/droid/DroidSans.ttf"))
            .unwrap();

        let scaled_font = font.as_scaled(PxScale::from(24.0));

        let mut glyphs = Vec::new();
        layout_glyphs(scaled_font, point(20.0, 20.0), "Hello World!", &mut glyphs);

        let height = scaled_font.height().ceil() as u32;
        let width = {
            let min_x = glyphs.first().unwrap().position.x;
            let last_glyph = glyphs.last().unwrap();
            let max_x = last_glyph.position.x + scaled_font.h_advance(last_glyph.id);
            // dbg!(max_x);
            // (max_x - min_x).ceil() as u32
            max_x.ceil() as u32
        };

        dbg!(height, width);

        let mut image: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(width + 40, height + 40);

        for glyph in glyphs {
            if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph) {
                let bounds = outlined_glyph.px_bounds();
                dbg!(bounds);

                outlined_glyph.draw(|x, y, alpha| {
                    let pixel = (u8::MAX as f32 * alpha) as u8;

                    image.put_pixel(
                        bounds.min.x as u32 + x,
                        bounds.min.y as u32 + y,
                        Luma([pixel]),
                    );
                });
            }
        }

        image.save("test.png").unwrap();
    }
}

fn layout_glyphs<SF: ScaleFont<F>, F: Font>(
    font: SF,
    position: Point,
    text: &str,
    target: &mut Vec<Glyph>,
) {
    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());

    let mut last_glyph: Option<Glyph> = None;

    for ch in text.chars() {
        if ch.is_control() {
            // TODO: handle newlines
            continue;
        }

        let mut glyph = font.scaled_glyph(ch);
        if let Some(prev) = last_glyph.take() {
            caret.x += font.kern(prev.id, glyph.id);
        }

        glyph.position = caret;

        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if !ch.is_whitespace() {
            caret = point(caret.x + v_advance, position.y);
            glyph.position = caret;
            last_glyph = None;
        }

        target.push(glyph);
    }
}
