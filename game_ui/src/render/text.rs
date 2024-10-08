use std::borrow::Borrow;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::ops::Range;
use std::sync::OnceLock;

use ab_glyph::{point, Font, FontRef, Glyph, Point, PxScale, ScaleFont};
use game_common::collections::lru::LruCache;
use game_tracing::trace_span;
use glam::{UVec2, Vec2};
use image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use parking_lot::Mutex;
use wgpu::hal::auxil::db;

use super::image::Image;
use crate::layout::computed_style::{ComputedBounds, ComputedStyle};
use crate::style::Color;

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
    fn new(
        text: String,
        size: u32,
        max: UVec2,
        caret: Option<u32>,
        selection_range: Option<Range<usize>>,
        selection_color: Color,
    ) -> Self {
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
                bounds: max,
                caret,
                selection_range,
                selection_color,
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

impl Debug for OwnedKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.key, f)
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BorrowedKey<'a> {
    text: &'a str,
    size: u32,
    bounds: UVec2,
    caret: Option<u32>,
    selection_range: Option<Range<usize>>,
    selection_color: Color,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub size: f32,
    pub caret: Option<u32>,
    pub selection_range: Option<Range<usize>>,
    pub selection_color: Color,
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
            selection_range: None,
            selection_color: Color::BLACK,
        }
    }

    pub(crate) fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        let image = render_to_texture(
            &self.text,
            self.size,
            UVec2::splat(0),
            self.caret,
            self.selection_range.clone(),
            self.selection_color,
        );
        Image { image }.bounds(style)
    }

    pub(crate) fn render_to_texture(&self, bounds: UVec2) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let key = BorrowedKey {
            text: &self.text,
            size: self.size.to_bits(),
            bounds,
            caret: self.caret,
            selection_range: self.selection_range.clone(),
            selection_color: self.selection_color,
        };

        if let Some(res) = TEXT_CACHE
            .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
            .lock()
            .get(&key)
        {
            return res.clone();
        }

        let font = FontRef::try_from_slice(DEFAULT_FONT).unwrap();

        // let scaled_font = font.as_scaled(PxScale::from(size));

        todo!()
    }
}

pub(crate) fn render_to_texture(
    text: &str,
    size: f32,
    max: UVec2,
    // Position of the caret where 0 is before the first character.
    caret: Option<u32>,
    selection_range: Option<Range<usize>>,
    selection_color: Color,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let key = BorrowedKey {
        text,
        size: size.to_bits(),
        bounds: max,
        caret,
        selection_range: selection_range.clone(),
        selection_color,
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
    let image_size = layout_glyphs(scaled_font, text, max.as_vec2(), &mut glyphs);

    let mut image = RgbaImage::new(image_size.x, image_size.y);

    for (index, glyph) in glyphs.iter().enumerate() {
        if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph.clone()) {
            let bounds = outlined_glyph.px_bounds();

            if let Some(selection_range) = &selection_range {
                if selection_range.contains(&index) {
                    for x in bounds.min.x.floor() as u32..bounds.max.x.ceil() as u32 {
                        for y in bounds.min.y.floor() as u32..bounds.max.y.ceil() as u32 {
                            image.put_pixel(x, y, selection_color.0);
                        }
                    }
                }
            }

            outlined_glyph.draw(|x, y, cov| {
                let pixel = (cov * 255.0) as u8;

                if let Some(px) =
                    image.get_pixel_mut_checked(bounds.min.x as u32 + x, bounds.min.y as u32 + y)
                {
                    px.blend(&Rgba([pixel; 4]));
                }
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
        OwnedKey::new(
            text.to_owned(),
            size.to_bits(),
            max,
            caret,
            selection_range,
            selection_color,
        ),
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
    bounds: Vec2,
    target: &mut Vec<Glyph>,
) -> UVec2 {
    let _span = trace_span!("layout_glyphs").entered();

    let mut max_line_width = 0.0;

    let v_advance = font.height() + font.line_gap();
    let mut caret = point(0.0, font.ascent());

    let mut last_glyph: Option<Glyph> = None;

    for ch in text.chars() {
        if ch.is_control() {
            if ch == '\n' {
                max_line_width = f32::max(max_line_width, caret.x);
                caret = point(0.0, caret.y + v_advance);
                last_glyph = None;
            }

            continue;
        }

        let mut glyph = font.scaled_glyph(ch);
        let h_advance = font.h_advance(glyph.id);

        let kern = last_glyph
            .take()
            .map(|last_glyph| font.kern(last_glyph.id, glyph.id))
            .unwrap_or_default();

        // If the new char would exceeds the bounds put it
        // onto the next line.
        if caret.x + h_advance + kern > bounds.x {
            max_line_width = f32::max(max_line_width, caret.x);
            caret = point(0.0, caret.y + v_advance);
            glyph.position = caret;
        } else {
            glyph.position = caret;
            caret.x += h_advance + kern;
        }

        target.push(glyph);
    }

    max_line_width = f32::max(max_line_width, caret.x);

    debug_assert!(font.descent().is_sign_negative());

    UVec2 {
        x: max_line_width.ceil() as u32,
        y: (caret.y - font.descent()).ceil() as u32,
    }
}

pub(crate) fn get_position_in_text(text: &str, size: f32, max: UVec2, cursor: UVec2) -> usize {
    let font = FontRef::try_from_slice(DEFAULT_FONT).unwrap();
    let scaled_font = font.as_scaled(PxScale::from(size));

    let mut glyphs = Vec::new();
    layout_glyphs(scaled_font, text, max.as_vec2(), &mut glyphs);

    for (index, glyphs) in glyphs.windows(2).enumerate() {
        let Some(a) = glyphs.get(0) else {
            break;
        };
        let Some(b) = glyphs.get(1) else {
            break;
        };

        let a_pos = UVec2::new(a.position.x as u32, a.position.y as u32);
        let b_pos = UVec2::new(b.position.x as u32, b.position.y as u32);

        let Some(outlined_a) = scaled_font.outline_glyph(a.clone()) else {
            continue;
        };

        if cursor.x < b_pos.x {
            let middle_x = (b_pos.x - a_pos.x) / 2 + a_pos.x;
            let max_y = outlined_a.px_bounds().max.y as u32;

            if cursor.y > max_y {
                continue;
            }

            if cursor.x < middle_x {
                return index;
            } else {
                return index + 1;
            }
        }
    }

    glyphs.len()
}

#[cfg(test)]
mod tests {
    use ab_glyph::{Font, FontRef, PxScale};
    use game_common::collections::lru::LruCache;
    use glam::{UVec2, Vec2};
    use parking_lot::Mutex;

    use crate::style::Color;

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
        let max = UVec2::MAX;

        render_to_texture(text, size, max, None, None, Color::default());
    }

    #[test]
    fn render_to_texture_newline() {
        let text = "abcdefghijklmnopqrstuvwxyz\nabcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let max = UVec2::MAX;

        render_to_texture(text, size, max, None, None, Color::default());
    }

    #[test]
    fn render_to_texture_overflow() {
        let text: String = (0..1000).map(|_| "a").collect();
        let size = 10.0;
        let max = UVec2::splat(1000);

        render_to_texture(&text, size, max, None, None, Color::default());
    }

    #[test]
    fn layout_glyphs_width_too_small() {
        let font = test_font();
        let font = font.as_scaled(PxScale::from(100.0));
        let text = "Hello";
        let bounds = Vec2::splat(1.0);
        let mut target = Vec::new();

        layout_glyphs(font, text, bounds, &mut target);
    }

    #[test]
    fn text_cache_get() {
        let text = "Hello World";
        let size: f32 = 24.0;
        let max = UVec2::splat(128);

        let key = BorrowedKey {
            text,
            size: size.to_bits(),
            bounds: max,
            caret: None,
            selection_range: None,
            selection_color: Color::default(),
        };

        assert!(TEXT_CACHE
            .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
            .lock()
            .get(&key)
            .is_none());

        // Call render twice, the second call will hit the cache.
        render_to_texture(text, size, max, None, None, Color::default());
        render_to_texture(text, size, max, None, None, Color::default());

        assert!(TEXT_CACHE
            .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
            .lock()
            .get(&key)
            .is_some());
    }
}
