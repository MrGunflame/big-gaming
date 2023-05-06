use glam::Vec2;

#[derive(Copy, Clone, Debug, Default)]
pub struct Style {
    pub position: Position,
    pub direction: Direction,
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
