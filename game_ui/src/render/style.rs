use bevy_ecs::system::Res;
use glam::Vec2;
use image::{ImageBuffer, Rgba};
use thiserror::Error;

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub bounds: Bounds,
    pub position: Position,
    pub direction: Direction,
    pub growth: Growth,
    pub background: Background,
    pub color: Color,
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
    #[default]
    Relative,
    Absolute(Vec2),
}

impl Position {
    #[inline]
    pub const fn is_relative(self) -> bool {
        matches!(self, Self::Relative)
    }

    #[inline]
    pub const fn is_absolute(self) -> bool {
        matches!(self, Self::Absolute(_))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Size {
    Pixels(f32),
    /// Viewport width percentage
    ViewportWidth(f32),
    /// Viewport height percentage
    ViewportHeight(f32),
}

impl Size {
    pub(crate) fn to_pixels(self, viewport: Vec2) -> f32 {
        match self {
            Self::Pixels(val) => val,
            Self::ViewportWidth(factor) => viewport.x * factor,
            Self::ViewportHeight(factor) => viewport.y * factor,
        }
    }
}

// TODO: Maybe replace with nalgebra vector.
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

impl Default for Bounds {
    fn default() -> Self {
        Self {
            min: SizeVec2::splat(Size::Pixels(0.0)),
            max: SizeVec2::splat(Size::Pixels(f32::INFINITY)),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Growth(pub Option<f32>);

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
    pub fn from_hex(s: &str) -> Result<Self, FromHexError> {
        Color::from_hex(s).map(|c| Self::Color(c.0))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub Rgba<u8>);

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

    pub fn from_hex(s: &str) -> Result<Self, FromHexError> {
        let bytes = hex::decode(s)?;

        let r = *bytes.get(0).ok_or(FromHexError::InvalidLength)?;
        let g = *bytes.get(1).ok_or(FromHexError::InvalidLength)?;
        let b = *bytes.get(2).ok_or(FromHexError::InvalidLength)?;
        let a = 255;

        Ok(Self(Rgba([r, g, b, a])))
    }
}

#[derive(Clone, Debug, Error)]
pub enum FromHexError {
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    #[error("invalid length")]
    InvalidLength,
}
