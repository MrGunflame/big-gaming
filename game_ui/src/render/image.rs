mod overlay;

use game_tracing::trace_span;
use glam::UVec2;
use image::imageops::FilterType;
use image::{ImageBuffer, Pixel, Rgba};

use self::overlay::overlay_unchecked;

use super::debug::is_debug_render_enabled;
use crate::layout::computed_style::{ComputedBorderRadius, ComputedBounds, ComputedStyle};
use crate::style::Background;

#[derive(Clone, Debug)]
pub struct Image {
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl Image {
    pub(crate) fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        let width = style.padding.left + style.padding.right;
        let height = style.padding.top + style.padding.bottom;

        let size = UVec2::new(self.image.width() + width, self.image.height() + height);

        ComputedBounds {
            min: size,
            max: size,
        }
    }
}

pub fn apply_border(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, style: &ComputedStyle) {
    let _span = trace_span!("image::apply_border").entered();

    if style.border.top == 0
        && style.border.bottom == 0
        && style.border.left == 0
        && style.border.right == 0
    {
        return;
    }

    let pixel = style.style.border.color.0;

    // If the border on any side is greater than the image on that side, the
    // result is equivalent to applying the border to all pixels in the image.
    if style.border.top >= img.height()
        || style.border.bottom >= img.height()
        || style.border.left >= img.width()
        || style.border.right >= img.width()
    {
        for px in img.pixels_mut() {
            px.blend(&pixel);
        }

        return;
    }

    // Note that for transparent border values to function correctly we must
    // not blend a pixel at any position twice.
    // This means that for every side of the border we must skip pixels already
    // accessed by a previous border segment.

    // Top
    let top_start = 0;
    let top_end = u32::min(style.border.top, img.height());
    for y in top_start..top_end {
        for x in 0..img.width() {
            img.get_pixel_mut(x, y).blend(&pixel);
        }
    }

    // Bottom
    // Note: The above check validates that this subtraction can never underflow.
    let bottom_start = u32::max(img.height() - style.border.bottom, top_end);
    let bottom_end = img.height();
    for y in bottom_start..bottom_end {
        for x in 0..img.width() {
            img.get_pixel_mut(x, y).blend(&pixel);
        }
    }

    // To prevent accessing a pixel from the top or bottom borders we
    // limit ourselves to the area that is not covered by those borders.
    let y_start = top_end;
    let y_end = bottom_start;

    // Left
    let left_start = 0;
    let left_end = u32::min(style.border.left, img.width());
    for x in left_start..left_end {
        for y in y_start..y_end {
            img.get_pixel_mut(x, y).blend(&pixel);
        }
    }

    // Right
    // Note: The above check validates that this subtraction can never underflow.
    let right_start = u32::max(img.width() - style.border.right, left_end);
    let right_end = img.width();
    for x in right_start..right_end {
        for y in y_start..y_end {
            img.get_pixel_mut(x, y).blend(&pixel);
        }
    }
}

pub fn apply_background(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, style: &ComputedStyle) {
    let _span = trace_span!("image::apply_background").entered();

    let width = img.width() + style.padding.left + style.padding.right;
    let height = img.height() + style.padding.top + style.padding.bottom;

    match &style.style.background {
        Background::None => {
            if style.padding.top == 0
                && style.padding.bottom == 0
                && style.padding.left == 0
                && style.padding.right == 0
            {
                return;
            }

            let mut buffer = ImageBuffer::new(width, height);

            let start_x = style.padding.left;
            let start_y = style.padding.top;

            for x in start_x..start_x + img.width() {
                for y in start_y..start_y + img.height() {
                    let px = img.get_pixel(x - start_x, y - start_y);

                    buffer.put_pixel(x, y, *px);
                }
            }

            *img = buffer;
        }
        Background::Color(color) => {
            let size = (width as usize)
                .checked_mul(height as usize)
                .and_then(|r| r.checked_mul(std::mem::size_of::<Rgba<u8>>()))
                .unwrap();

            let mut buf: Vec<u8> = Vec::with_capacity(size);
            unsafe {
                // Note that if the above statement doesn't overflow,
                // this wont either.
                let pixels = width as usize * height as usize;

                for index in 0..pixels {
                    let offset = index * std::mem::size_of::<Rgba<u8>>();

                    let ptr = buf.as_mut_ptr().add(offset);

                    std::ptr::write(ptr.cast::<[u8; 4]>(), color.0);
                }

                buf.set_len(size);
            }

            let mut buffer = ImageBuffer::from_raw(width, height, buf).unwrap();

            unsafe {
                overlay_unchecked(&mut buffer, img, style.padding.left, style.padding.top);
            };

            *img = buffer;
        }
        Background::Image(image) => {
            let mut buffer = image::imageops::resize(image, width, height, FilterType::Nearest);

            unsafe {
                overlay_unchecked(&mut buffer, img, style.padding.left, style.padding.top);
            }

            *img = buffer;
        }
    }
}

pub fn apply_border_radius(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    border_radius: ComputedBorderRadius,
) {
    let _span = trace_span!("image::apply_border_radius").entered();

    if img.width() == 0 || img.height() == 0 {
        return;
    }

    let pixel = if is_debug_render_enabled() {
        Rgba([255, 255, 0, 255])
    } else {
        Rgba([0, 0, 0, 0])
    };

    // Top left
    let start = UVec2::new(0, 0);
    let end = UVec2::new(
        start.x + border_radius.top_left,
        start.y + border_radius.top_left,
    );

    for x in start.x..end.x {
        for y in start.y..end.y {
            let distance = (end.as_vec2() - UVec2::new(x, y).as_vec2()).length();

            if distance as u32 > border_radius.top_left {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }

    // Bottom left
    let start = UVec2::new(0, img.height());
    let end = UVec2::new(
        start.x + border_radius.bottom_left,
        start.y.saturating_sub(border_radius.bottom_left + 1),
    );

    for x in start.x..end.x {
        for y in end.y..start.y {
            let distance = (end.as_vec2() - UVec2::new(x, y).as_vec2()).length();

            if distance as u32 > border_radius.bottom_left {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }

    // Top right
    let start = UVec2::new(img.width(), 0);
    let end = UVec2::new(
        start.x.saturating_sub(border_radius.top_right + 1),
        start.y + border_radius.top_right,
    );

    for x in end.x..start.x {
        for y in start.y..end.y {
            let distance = (end.as_vec2() - UVec2::new(x, y).as_vec2()).length();

            if distance as u32 > border_radius.top_right {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }

    // Bottom right
    let start = UVec2::new(img.width(), img.height());
    let end = UVec2::new(
        start.x.saturating_sub(border_radius.bottom_right + 1),
        start.y.saturating_sub(border_radius.bottom_right + 1),
    );

    for x in end.x..start.x {
        for y in end.y..start.y {
            let distance = (end.as_vec2() - UVec2::new(x, y).as_vec2()).length();

            if distance as u32 > border_radius.bottom_right {
                if let Some(p) = img.get_pixel_mut_checked(x, y) {
                    *p = pixel;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::fmt::Debug;

    use glam::UVec2;
    use image::{GenericImageView, ImageBuffer, Pixel, Rgba};

    use crate::layout::computed_style::{ComputedBorderRadius, ComputedStyle};
    use crate::style::{Background, Border, Color, Padding, Size, Style};

    use super::{apply_background, apply_border, apply_border_radius};

    /// Asserts that image `a` contains image `b`, starting at position `(x, y)`.
    #[track_caller]
    fn assert_sub_image<T, U, I, J, P>(a: T, b: U, x: u32, y: u32)
    where
        T: Borrow<I>,
        U: Borrow<J>,
        I: GenericImageView<Pixel = P>,
        J: GenericImageView<Pixel = P>,
        P: Pixel + Debug + PartialEq,
    {
        let a = a.borrow();
        let b = b.borrow();

        assert!(a.width() >= b.width());
        assert!(a.height() >= b.height());

        let start_x = x;
        let start_y = y;
        let end_x = start_x + b.width();
        let end_y = start_y + b.height();

        for x in start_x..end_x {
            for y in start_y..end_y {
                let src_px = a.get_pixel(x, y);
                let dst_px = b.get_pixel(x - start_x, y - start_y);

                assert_eq!(src_px, dst_px);
            }
        }
    }

    #[test]
    fn background_none_no_padding() {
        let viewport = UVec2::splat(1000);

        let image = ImageBuffer::new(100, 100);

        let style = ComputedStyle::new(
            Style {
                background: Background::None,
                padding: Padding::NONE,
                ..Default::default()
            },
            viewport,
        );

        let src = image.clone();
        let mut out = image;
        apply_background(&mut out, &style);

        assert_eq!(out.width(), src.width());
        assert_eq!(out.height(), src.height());

        assert_eq!(src, out);
    }

    #[test]
    fn background_none_padding() {
        let viewport = UVec2::splat(1000);

        let image = ImageBuffer::new(100, 100);

        let style = ComputedStyle::new(
            Style {
                background: Background::None,
                padding: Padding::splat(Size::Pixels(2)),
                ..Default::default()
            },
            viewport,
        );

        let src = image.clone();
        let mut out = image;
        apply_background(&mut out, &style);

        assert_eq!(out.width(), 100 + 4);
        assert_eq!(out.height(), 100 + 4);

        assert_sub_image(out, src, 2, 2);
    }

    #[test]
    fn background_color_no_padding() {
        let viewport = UVec2::splat(1000);
        let color = Rgba([123, 124, 125, 126]);

        let image = ImageBuffer::new(100, 100);

        let style = ComputedStyle::new(
            Style {
                background: Background::Color(color),
                ..Default::default()
            },
            viewport,
        );

        let src = image.clone();
        let mut out = image;
        apply_background(&mut out, &style);

        assert_eq!(out.width(), src.width());
        assert_eq!(out.height(), src.height());
    }

    #[test]
    fn background_color_padding() {
        let viewport = UVec2::splat(1000);
        let color = Rgba([123, 124, 125, 126]);

        let image = ImageBuffer::from_pixel(100, 100, Rgba([0, 0, 0, 255]));

        let style = ComputedStyle::new(
            Style {
                background: Background::Color(color),
                padding: Padding::splat(Size::Pixels(2)),
                ..Default::default()
            },
            viewport,
        );

        let src = image.clone();
        let mut out = image;
        apply_background(&mut out, &style);

        assert_eq!(out.width(), src.width() + 4);
        assert_eq!(out.height(), src.height() + 4);

        assert_sub_image(out, src, 2, 2);
    }

    #[test]
    fn background_image_no_padding() {
        let viewport = UVec2::splat(1000);
        let bg = ImageBuffer::from_pixel(100, 100, Rgba([123, 124, 125, 126]));

        let image = ImageBuffer::new(100, 100);

        let style = ComputedStyle::new(
            Style {
                background: Background::Image(bg),
                padding: Padding::NONE,
                ..Default::default()
            },
            viewport,
        );

        let src = image.clone();
        let mut out = image;
        apply_background(&mut out, &style);

        assert_eq!(out.width(), src.width());
        assert_eq!(out.height(), src.height());
    }

    #[test]
    fn background_image_padding() {
        let viewport = UVec2::splat(1000);
        let bg = ImageBuffer::from_pixel(100, 100, Rgba([123, 124, 125, 126]));

        let image = ImageBuffer::from_pixel(100, 100, Rgba([0, 0, 0, 255]));

        let style = ComputedStyle::new(
            Style {
                background: Background::Image(bg),
                padding: Padding::splat(Size::Pixels(2)),
                ..Default::default()
            },
            viewport,
        );

        let src = image.clone();
        let mut out = image;
        apply_background(&mut out, &style);

        assert_eq!(out.width(), src.width() + 4);
        assert_eq!(out.height(), src.height() + 4);

        assert_sub_image(out, src, 2, 2);
    }

    #[test]
    fn border_radius() {
        let mut image = ImageBuffer::new(100, 100);
        let border_radius = ComputedBorderRadius {
            top_left: 10,
            bottom_left: 10,
            top_right: 10,
            bottom_right: 10,
        };

        apply_border_radius(&mut image, border_radius);
    }

    #[test]
    fn border_radius_image_empty() {
        let mut image = ImageBuffer::new(0, 0);
        let border_radius = ComputedBorderRadius {
            top_left: 10,
            bottom_left: 10,
            top_right: 10,
            bottom_right: 10,
        };

        apply_border_radius(&mut image, border_radius);
    }

    #[test]
    fn border_radius_image_too_small() {
        let mut image = ImageBuffer::new(10, 10);
        let border_radius = ComputedBorderRadius {
            top_left: 20,
            bottom_left: 20,
            top_right: 20,
            bottom_right: 20,
        };

        apply_border_radius(&mut image, border_radius);
    }

    #[test]
    fn border_out_of_bounds() {
        let mut image = ImageBuffer::from_pixel(10, 10, Color::WHITE.0);
        let style = ComputedStyle::new(
            Style {
                border: Border::splat(Size::Pixels(100), Color::BLACK),
                ..Default::default()
            },
            UVec2::ZERO,
        );

        apply_border(&mut image, &style);
    }
}
