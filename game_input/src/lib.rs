#![deny(unsafe_op_in_unsafe_fn)]

pub mod emulator;
pub mod hotkeys;
pub mod keyboard;
pub mod mouse;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    #[inline]
    pub const fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }

    pub const fn is_released(self) -> bool {
        matches!(self, Self::Released)
    }
}
