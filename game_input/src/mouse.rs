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
    Other(u16),
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
