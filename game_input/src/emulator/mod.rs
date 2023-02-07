//! Emulator for input devices
//!
//! Mostly useful for testing.

mod keyboard;
mod mouse;

pub use keyboard::{KeyboardEmulator, KeyboardEmulatorPlugin};
pub use mouse::{MouseEmulator, MouseEmulatorPlugin};
