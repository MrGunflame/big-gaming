use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::sync::OnceLock;

use ab_glyph::{point, Font, FontRef, Glyph, Point, PxScale, ScaleFont};
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

static TEXT_CACHE: OnceLock<Mutex<LruCache<OwnedKey, ImageBuffer<Rgba<u8>, Vec<u8>>>>> =
    OnceLock::new();

struct OwnedKey {
    // To implement Borrow<BorrowedKey> we decompose the string
    // into their raw parts and store a static reference in `key`.
    ptr: *mut u8,
    len: usize,
    cap: usize,
    // Note that `key` MUST be dropped before the string buffer
    // itself is dropped, hence it is `ManuallyDrop`.
    key: ManuallyDrop<BorrowedKey<'static>>,
}

impl OwnedKey {
    fn new(text: String, size: u32, max: UVec2, caret: Option<u32>) -> Self {
        let ptr = text.as_ptr().cast_mut();
        let len = text.len();
        let cap = text.capacity();

        let text = text.leak();

        Self {
            ptr,
            len,
            cap,
            key: ManuallyDrop::new(BorrowedKey {
                text,
                size,
                max,
                caret,
            }),
        }
    }
}

impl Drop for OwnedKey {
    fn drop(&mut self) {
        // SAFETY: `ptr`, `len` and `cap` were previously created by decomposing
        // a `String` into their raw parts.
        // We drop `self.key` before dropped the string buffer, which guarantees
        // that all references to the underlying buffer are dropped.
        unsafe {
            ManuallyDrop::drop(&mut self.key);
            drop(String::from_raw_parts(self.ptr, self.len, self.cap));
        }
    }
}

impl<'a> Borrow<BorrowedKey<'a>> for OwnedKey {
    #[inline]
    fn borrow(&self) -> &BorrowedKey<'a> {
        &self.key
    }
}

impl PartialEq for OwnedKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for OwnedKey {}

impl Hash for OwnedKey {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.key.hash(state);
    }
}

// SAFETY: It is safe to send string references across threads
// and to drop the string on a different thread.
unsafe impl Send for OwnedKey {}
unsafe impl Sync for OwnedKey {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct BorrowedKey<'a> {
    text: &'a str,
    size: u32,
    max: UVec2,
    caret: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub size: f32,
    pub caret: Option<u32>,
}

impl Text {
    #[inline]
    pub fn new<T>(text: T, size: f32, caret: Option<u32>) -> Self
    where
        T: ToString,
    {
        Self {
            text: text.to_string(),
            size,
            caret,
        }
    }

    pub(crate) fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        let image = render_to_texture(&self.text, self.size, UVec2::splat(0), self.caret);
        Image { image }.bounds(style)
    }
}

impl DrawElement for Text {
    fn draw(&self, style: &ComputedStyle, layout: super::Rect, size: UVec2) -> Option<DrawCommand> {
        let image = render_to_texture(&self.text, self.size, layout.max - layout.min, self.caret);

        Image { image }.draw(style, layout, size)
    }
}

pub(crate) fn render_to_texture(
    text: &str,
    size: f32,
    max: UVec2,
    // Position of the caret where 0 is before the first character.
    caret: Option<u32>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let key = BorrowedKey {
        text,
        size: size.to_bits(),
        max,
        caret,
    };

    if let Some(res) = TEXT_CACHE
        .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
        .lock()
        .get(&key)
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

    for glyph in &glyphs {
        if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph.clone()) {
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

    // Render the caret.
    if let Some(caret) = caret {
        let caret = get_caret(scaled_font, &glyphs, caret as usize);

        for x in caret.position.x as u32..caret.position.x as u32 + caret.width as u32 {
            for y in caret.position.y as u32..caret.position.y as u32 + caret.height as u32 {
                if let Some(pixel) = image.get_pixel_mut_checked(x, y) {
                    *pixel = Rgba([255, 255, 255, 255]);
                }
            }
        }
    }

    TEXT_CACHE.get().unwrap().lock().insert(
        OwnedKey::new(text.to_owned(), size.to_bits(), max, caret),
        image.clone(),
    );
    image
}

fn get_caret<SF, F>(font: SF, glyphs: &[Glyph], index: usize) -> Caret
where
    SF: ScaleFont<F>,
    F: Font,
{
    let _span = trace_span!("get_caret").entered();

    let position = match glyphs.get(index) {
        Some(glyph) => {
            let mut pos = glyph.position;
            pos.y -= font.ascent();
            pos
        }
        None => match glyphs.last() {
            Some(glyph) => {
                let mut pos = glyph.position;
                pos.x += font.h_advance(glyph.id);
                pos.y -= font.ascent();
                pos
            }
            None => Point { x: 0.0, y: 0.0 },
        },
    };

    let height = font.height();
    let width = 2.0;

    Caret {
        position,
        height,
        width,
    }
}

#[derive(Copy, Clone, Debug)]
struct Caret {
    position: Point,
    height: f32,
    width: f32,
}

fn layout_glyphs<SF: ScaleFont<F>, F: Font>(
    font: SF,
    text: &str,
    max_width: f32,
    target: &mut Vec<Glyph>,
) -> (u32, f32) {
    let _span = trace_span!("layout_glyphs").entered();

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
    use game_common::collections::lru::LruCache;
    use glam::UVec2;
    use parking_lot::Mutex;

    use super::{
        layout_glyphs, render_to_texture, BorrowedKey, DEFAULT_FONT, TEXT_CACHE, TEXT_CACHE_CAP,
    };

    fn test_font() -> FontRef<'static> {
        FontRef::try_from_slice(DEFAULT_FONT).unwrap()
    }

    #[test]
    fn render_to_texture_singleline() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let max = UVec2::splat(0);

        render_to_texture(text, size, max, None);
    }

    #[test]
    fn render_to_texture_newline() {
        let text = "abcdefghijklmnopqrstuvwxyz\nabcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let max = UVec2::splat(0);

        render_to_texture(text, size, max, None);
    }

    #[test]
    fn render_to_texture_overflow() {
        let text: String = (0..1000).map(|_| "a").collect();
        let size = 10.0;
        let max = UVec2::splat(0);

        render_to_texture(&text, size, max, None);
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

    #[test]
    fn text_cache_get() {
        let text = "Hello World";
        let size: f32 = 24.0;
        let max = UVec2::splat(128);

        let key = BorrowedKey {
            text,
            size: size.to_bits(),
            max,
            caret: None,
        };

        assert!(TEXT_CACHE
            .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
            .lock()
            .get(&key)
            .is_none());

        // Call render twice, the second call will hit the cache.
        render_to_texture(text, size, max, None);
        render_to_texture(text, size, max, None);

        assert!(TEXT_CACHE
            .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
            .lock()
            .get(&key)
            .is_some());
    }
}
