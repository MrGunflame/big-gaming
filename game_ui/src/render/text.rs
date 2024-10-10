use std::borrow::{Borrow, Cow};
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
pub struct Text<'a> {
    pub text: Cow<'a, str>,
    pub size: f32,
    pub caret: Option<u32>,
    pub selection_range: Option<Range<usize>>,
    pub selection_color: Color,
}

impl<'a> Text<'a> {
    #[inline]
    pub fn new<T>(text: T, size: f32, caret: Option<u32>) -> Self
    where
        T: ToString,
    {
        Self {
            text: Cow::Owned(text.to_string()),
            size,
            caret,
            selection_range: None,
            selection_color: Color::default(),
        }
    }

    pub(crate) fn as_ref(&self) -> Text<'_> {
        Text {
            text: Cow::Borrowed(&self.text),
            size: self.size,
            caret: self.caret,
            selection_range: self.selection_range.clone(),
            selection_color: self.selection_color,
        }
    }

    /// Returns the size of the the texture used to rasterize this `Text` without actually doing
    /// any rasterization.
    ///
    /// The returned size is equivalent to the size of the [`ImageBuffer`] returned by
    /// [`render_to_texture`].
    ///
    /// [`render_to_texture`]: Self::render_to_texture
    pub(crate) fn bounds(&self, bounds: UVec2) -> UVec2 {
        let font = FontRef::try_from_slice(DEFAULT_FONT).unwrap();
        let scaled_font = font.as_scaled(PxScale::from(self.size));
        layout_glyphs(scaled_font, &self.text, bounds.as_vec2(), &mut Vec::new())
    }

    /// Rasterizes this `Text` onto a texture with a `bounds` constraint.
    ///
    /// The returned [`ImageBuffer`] will never be bigger than `bounds`.
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

        let _span = trace_span!("text::render_to_texture").entered();

        let font = FontRef::try_from_slice(DEFAULT_FONT).unwrap();

        let scaled_font = font.as_scaled(PxScale::from(self.size));

        let mut glyphs = Vec::new();
        let image_size = layout_glyphs(scaled_font, &self.text, bounds.as_vec2(), &mut glyphs);

        let mut image = RgbaImage::new(image_size.x, image_size.y);

        if let Some(range) = &self.selection_range {
            draw_selection_range(
                scaled_font,
                &mut image,
                &self.text,
                &glyphs,
                range.clone(),
                self.selection_color,
            );
        }

        for glyph in glyphs.iter() {
            if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph.clone()) {
                let bounds = outlined_glyph.px_bounds();

                outlined_glyph.draw(|x, y, cov| {
                    let pixel = (cov * 255.0) as u8;

                    if let Some(px) = image
                        .get_pixel_mut_checked(bounds.min.x as u32 + x, bounds.min.y as u32 + y)
                    {
                        px.blend(&Rgba([pixel; 4]));
                    }
                });
            }
        }

        // Render the caret.
        if let Some(caret) = self.caret {
            let caret = get_caret(scaled_font, &self.text, &glyphs, caret as usize);

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
                self.text.to_string(),
                self.size.to_bits(),
                bounds,
                self.caret,
                self.selection_range.clone(),
                self.selection_color,
            ),
            image.clone(),
        );
        image
    }
}

fn draw_selection_range<SF, F>(
    font: SF,
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    text: &str,
    glyphs: &[Glyph],
    range: Range<usize>,
    color: Color,
) where
    SF: ScaleFont<F>,
    F: Font,
{
    let _span = trace_span!("draw_selection_range").entered();

    if range.is_empty() {
        return;
    }

    let Some(selected_text) = text.get(range.clone()) else {
        return;
    };

    let v_advance = font.height() + font.line_gap();

    // Since newlines are not rendered to glyphs we must
    // skip over any newline chars that exist before `index`.
    let offset = match text.get(..range.start) {
        Some(text) => text.chars().filter(|ch| *ch == '\n').count(),
        None => 0,
    };

    let mut line_offset = 0;
    for (line_index, line) in selected_text.split('\n').enumerate() {
        let start = line_offset;
        let end = line.len() + start;
        line_offset += line.len() + 1;

        // For every line we iterate over there is a newline char
        // that needs to be skipped.
        let offset = offset + line_index;

        // Map the text line to the list of glyphs in the line.
        let line_glyphs = &glyphs[range.start + start - offset..range.start + end - offset];

        let start_x = match line_glyphs.first() {
            Some(glyph) => glyph.position.x as u32,
            None => 0,
        };
        let end_x = match line_glyphs.last() {
            Some(glyph) => (glyph.position.x + font.h_advance(glyph.id)) as u32,
            None => 0,
        };

        let min = UVec2 {
            x: start_x,
            y: (v_advance * offset as f32) as u32,
        };
        let max = UVec2 {
            x: end_x,
            y: (v_advance * offset as f32 + font.height()) as u32,
        };

        debug_assert!(max.x >= min.x);
        debug_assert!(max.y >= min.y);

        for x in min.x..max.x {
            for y in min.y..max.y {
                image.put_pixel(x, y, color.0);
            }
        }
    }
}

fn get_caret<SF, F>(font: SF, text: &str, glyphs: &[Glyph], index: usize) -> Caret
where
    SF: ScaleFont<F>,
    F: Font,
{
    let _span = trace_span!("get_caret").entered();

    let v_advance = font.height() + font.line_gap();

    // Since newlines are not rendered to glyphs we must
    // skip over any newline chars that exist before `index`.
    let offset = match text.get(..index) {
        Some(text) => text.chars().filter(|ch| *ch == '\n').count(),
        None => 0,
    };

    let mut position = Point {
        x: 0.0,
        y: v_advance * offset as f32,
    };

    let prev = index
        .checked_sub(1)
        .and_then(|prev_index| text.chars().nth(prev_index));

    let next = text.chars().nth(index);

    match next {
        // Both the next and previous chars are newlines.
        // This means were are in an empty line.
        Some('\n') if prev == Some('\n') => (),
        // The next char is a newline and the previous char is a normal glyph.
        // This means we are at the end of the line and need to set the caret
        // after the last glyph of the line.
        Some('\n') => {
            // Note that this operation can never underflow since there are always
            // `offset` newline chars before `next`. Together with `next` this makes
            // `offset + 1`.
            if let Some(glyph) = glyphs.get(index - offset - 1) {
                position.x = glyph.position.x + font.h_advance(glyph.id);
            }
        }
        // The next char is a normal glyph.
        // This means that we are in a line and need to the set the caret to the
        // glyph of `next`, placing it between `prev` and `next`.
        Some(_) => {
            if let Some(glyph) = glyphs.get(index - offset) {
                position.x = glyph.position.x;
            }
        }
        // There exists no next char and the previous char is a newline.
        // This means we are in an empty line at the end of `text`.
        None if prev == Some('\n') => (),
        // There exists no next char.
        // This means we are at the end of the `text` and must set the caret after
        // the last glyph in the `text`.
        None => {
            if let Some(glyph) = glyphs.last() {
                position.x = glyph.position.x + font.h_advance(glyph.id);
            }
        }
    }

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

    use super::{layout_glyphs, BorrowedKey, Text, DEFAULT_FONT, TEXT_CACHE, TEXT_CACHE_CAP};

    fn test_font() -> FontRef<'static> {
        FontRef::try_from_slice(DEFAULT_FONT).unwrap()
    }

    #[test]
    fn render_to_texture_singleline() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let bounds = UVec2::MAX;

        Text::new(text, size, None).render_to_texture(bounds);
    }

    #[test]
    fn render_to_texture_newline() {
        let text = "abcdefghijklmnopqrstuvwxyz\nabcdefghijklmnopqrstuvwxyz";
        let size = 100.0;
        let bounds = UVec2::MAX;

        Text::new(text, size, None).render_to_texture(bounds);
    }

    #[test]
    fn render_to_texture_overflow() {
        let text: String = (0..1000).map(|_| "a").collect();
        let size = 10.0;
        let bounds = UVec2::splat(1000);

        Text::new(text, size, None).render_to_texture(bounds);
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
        let bounds = UVec2::splat(128);

        let key = BorrowedKey {
            text,
            size: size.to_bits(),
            bounds,
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
        Text::new(text, size, None).render_to_texture(bounds);
        Text::new(text, size, None).render_to_texture(bounds);

        assert!(TEXT_CACHE
            .get_or_init(|| Mutex::new(LruCache::new(TEXT_CACHE_CAP)))
            .lock()
            .get(&key)
            .is_some());
    }
}
