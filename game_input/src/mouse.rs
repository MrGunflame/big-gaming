use glam::Vec2;

use crate::ButtonState;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MouseMotion {
    pub delta: Vec2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ButtonState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    #[inline]
    pub const fn is_left(self) -> bool {
        matches!(self, Self::Left)
    }

    #[inline]
    pub const fn is_right(self) -> bool {
        matches!(self, Self::Right)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MouseWheel {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MouseScrollUnit {
    Line,
    Pixel,
}
