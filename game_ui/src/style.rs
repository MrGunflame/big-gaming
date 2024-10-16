use game_common::hex::{self, FromHexError};
use glam::UVec2;
use image::ImageBuffer;

pub use image::Rgba;

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub bounds: Bounds,
    pub position: Position,
    pub direction: Direction,
    pub growth: Growth,
    pub background: Background,
    pub color: Color,
    pub justify: Justify,
    pub padding: Padding,
    pub border_radius: BorderRadius,
    pub border: Border,
}

/// Flow direction
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

impl Direction {
    #[inline]
    pub const fn is_row(&self) -> bool {
        matches!(self, Self::Row)
    }

    #[inline]
    pub const fn is_column(&self) -> bool {
        matches!(self, Self::Column)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum Position {
    /// The position is intered from the parent.
    #[default]
    Relative,
    /// Use the provided absolute screen coordinates.
    Absolute(UVec2),
}

impl Position {
    /// Returns `true` if this `Position` is [`Relative`].
    ///
    /// [`Relative`]: Self::Relative
    #[inline]
    pub const fn is_relative(self) -> bool {
        matches!(self, Self::Relative)
    }

    /// Returns `true` if this `Position` is [`Absolute`].
    ///
    /// [`Absolute`]: Self::Absolute
    #[inline]
    pub const fn is_absolute(self) -> bool {
        matches!(self, Self::Absolute(_))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Size {
    Pixels(u32),
    /// Viewport width percentage
    ViewportWidth(f32),
    /// Viewport height percentage
    ViewportHeight(f32),
}

impl Size {
    /// The `Size` representing zero, the smallest possible size.
    pub const ZERO: Self = Self::Pixels(0);

    /// The `Size` representing infinity, the maximum size.
    pub const INFINITY: Self = Self::Pixels(u32::MAX);

    /// Returns the `Size` as pixels.
    #[inline]
    pub(crate) fn to_pixels(self, viewport: UVec2, scale_factor: f64) -> u32 {
        match self {
            Self::Pixels(val) => (val as f64 * scale_factor).ceil() as u32,
            Self::ViewportWidth(factor) => viewport.x * factor.ceil() as u32,
            Self::ViewportHeight(factor) => viewport.y * factor.ceil() as u32,
        }
    }

    pub fn is_zero(self) -> bool {
        matches!(
            self,
            Self::Pixels(0) | Self::ViewportWidth(0.0) | Self::ViewportHeight(0.0)
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SizeVec2 {
    pub x: Size,
    pub y: Size,
}

impl SizeVec2 {
    pub const fn splat(size: Size) -> Self {
        Self { x: size, y: size }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Bounds {
    pub min: SizeVec2,
    pub max: SizeVec2,
}

impl Bounds {
    /// Creates a new `Bounds` with the given `min` value and an unbounded `max` value.
    #[inline]
    pub const fn from_min(min: SizeVec2) -> Self {
        Self {
            min,
            max: SizeVec2::splat(Size::INFINITY),
        }
    }

    #[inline]
    pub const fn from_max(max: SizeVec2) -> Self {
        Self {
            min: SizeVec2::splat(Size::Pixels(0)),
            max,
        }
    }

    #[inline]
    pub const fn exact(size: SizeVec2) -> Self {
        Self {
            min: size,
            max: size,
        }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            min: SizeVec2::splat(Size::Pixels(0)),
            max: SizeVec2::splat(Size::INFINITY),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Growth {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Growth {
    pub const NONE: Self = Self { x: None, y: None };

    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            x: Some(x),
            y: Some(y),
        }
    }

    pub const fn x(x: f32) -> Self {
        Self {
            x: Some(x),
            y: None,
        }
    }

    pub const fn y(y: f32) -> Self {
        Self {
            x: None,
            y: Some(y),
        }
    }

    pub const fn splat(factor: f32) -> Self {
        Self {
            x: Some(factor),
            y: Some(factor),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum Background {
    // Note: We have `None` separately since it is a common case
    // and doesn't require any pixel blending.
    #[default]
    None,
    Color(Rgba<u8>),
    Image(ImageBuffer<Rgba<u8>, Vec<u8>>),
}

impl Background {
    pub const BLACK: Self = Self::Color(Rgba([0x00, 0x00, 0x00, 0xFF]));
    pub const SILVER: Self = Self::Color(Rgba([0xC0, 0xC0, 0xC0, 0xFF]));
    pub const GRAY: Self = Self::Color(Rgba([0x80, 0x80, 0x80, 0xFF]));
    pub const WHITE: Self = Self::Color(Rgba([0xFF, 0xFF, 0xFF, 0xFF]));
    pub const MAROON: Self = Self::Color(Rgba([0x80, 0x00, 0x00, 0xFF]));
    pub const RED: Self = Self::Color(Rgba([0xFF, 0x00, 0x00, 0xFF]));
    pub const PURPLE: Self = Self::Color(Rgba([0x80, 0x00, 0x80, 0xFF]));
    pub const FUCHSIA: Self = Self::Color(Rgba([0xFF, 0x00, 0xFF, 0xFF]));
    pub const GREEN: Self = Self::Color(Rgba([0x00, 0x80, 0x00, 0xFF]));
    pub const LIME: Self = Self::Color(Rgba([0x00, 0xFF, 0x00, 0xFF]));
    pub const OLIVE: Self = Self::Color(Rgba([0x80, 0x80, 0x00, 0xFF]));
    pub const YELLOW: Self = Self::Color(Rgba([0xFF, 0xFF, 0x00, 0xFF]));
    pub const NAVY: Self = Self::Color(Rgba([0x00, 0x00, 0x80, 0xFF]));
    pub const BLUE: Self = Self::Color(Rgba([0x00, 0x00, 0xFF, 0xFF]));
    pub const TEAL: Self = Self::Color(Rgba([0x00, 0x80, 0x80, 0xFF]));
    pub const AQUA: Self = Self::Color(Rgba([0x00, 0xFF, 0xFF, 0xFF]));

    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    #[inline]
    pub const fn is_color(&self) -> bool {
        matches!(self, Self::Color(_))
    }

    #[inline]
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    pub fn from_hex(s: &str) -> Result<Self, FromHexError> {
        Color::from_hex(s).map(|c| Self::Color(c.0))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub Rgba<u8>);

impl Color {
    pub const BLACK: Self = Self(Rgba([0x00, 0x00, 0x00, 0xFF]));
    pub const SILVER: Self = Self(Rgba([0xC0, 0xC0, 0xC0, 0xFF]));
    pub const GRAY: Self = Self(Rgba([0x80, 0x80, 0x80, 0xFF]));
    pub const WHITE: Self = Self(Rgba([0xFF, 0xFF, 0xFF, 0xFF]));
    pub const MAROON: Self = Self(Rgba([0x80, 0x00, 0x00, 0xFF]));
    pub const RED: Self = Self(Rgba([0xFF, 0x00, 0x00, 0xFF]));
    pub const PURPLE: Self = Self(Rgba([0x80, 0x00, 0x80, 0xFF]));
    pub const FUCHSIA: Self = Self(Rgba([0xFF, 0x00, 0xFF, 0xFF]));
    pub const GREEN: Self = Self(Rgba([0x00, 0x80, 0x00, 0xFF]));
    pub const LIME: Self = Self(Rgba([0x00, 0xFF, 0x00, 0xFF]));
    pub const OLIVE: Self = Self(Rgba([0x80, 0x80, 0x00, 0xFF]));
    pub const YELLOW: Self = Self(Rgba([0xFF, 0xFF, 0x00, 0xFF]));
    pub const NAVY: Self = Self(Rgba([0x00, 0x00, 0x80, 0xFF]));
    pub const BLUE: Self = Self(Rgba([0x00, 0x00, 0xFF, 0xFF]));
    pub const TEAL: Self = Self(Rgba([0x00, 0x80, 0x80, 0xFF]));
    pub const AQUA: Self = Self(Rgba([0x00, 0xFF, 0xFF, 0xFF]));
}

impl Default for Color {
    fn default() -> Self {
        Self(Rgba([255, 255, 255, 255]))
    }
}

impl Color {
    #[inline]
    pub(crate) fn to_f32(self) -> [f32; 4] {
        let r = (self.0 .0[0] as f32) / 255.0;
        let g = (self.0 .0[1] as f32) / 255.0;
        let b = (self.0 .0[2] as f32) / 255.0;
        let a = (self.0 .0[3] as f32) / 255.0;
        [r, g, b, a]
    }

    pub const fn from_hex(s: &str) -> Result<Self, FromHexError> {
        match hex::decode_to_array(s) {
            Ok([r, g, b]) => Ok(Self(Rgba([r, g, b, 255]))),
            Err(err) => Err(err),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Justify {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Padding {
    pub top: Size,
    pub bottom: Size,
    pub left: Size,
    pub right: Size,
}

impl Padding {
    pub const NONE: Self = Self {
        top: Size::ZERO,
        bottom: Size::ZERO,
        left: Size::ZERO,
        right: Size::ZERO,
    };

    pub const fn splat(size: Size) -> Self {
        Self {
            top: size,
            bottom: size,
            left: size,
            right: size,
        }
    }
}

impl Default for Padding {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Border {
    pub top: Size,
    pub bottom: Size,
    pub left: Size,
    pub right: Size,
    pub color: Color,
}

impl Default for Border {
    fn default() -> Self {
        Self::splat(Size::Pixels(0), Color::BLACK)
    }
}

impl Border {
    /// Creates a new `Border` with all sides set to the given values.
    pub const fn splat(size: Size, color: Color) -> Self {
        Self {
            top: size,
            bottom: size,
            left: size,
            right: size,
            color,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.top.is_zero() && self.bottom.is_zero() && self.left.is_zero() && self.right.is_zero()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BorderRadius {
    pub top_left: Size,
    pub bottom_left: Size,
    pub top_right: Size,
    pub bottom_right: Size,
}

impl BorderRadius {
    pub const fn splat(size: Size) -> Self {
        Self {
            top_left: size,
            bottom_left: size,
            top_right: size,
            bottom_right: size,
        }
    }
}

impl Default for BorderRadius {
    fn default() -> Self {
        Self {
            top_left: Size::ZERO,
            bottom_left: Size::ZERO,
            top_right: Size::ZERO,
            bottom_right: Size::ZERO,
        }
    }
}
