use glam::Vec2;

#[derive(Copy, Clone, Debug, Default)]
pub struct Style {
    pub bounds: Bounds,
    pub position: Position,
    pub direction: Direction,
    pub growth: Growth,
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
