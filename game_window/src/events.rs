use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseMotion, MouseWheel};
use glam::Vec2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindowEvent {
    WindowCreated(WindowCreated),
    WindowResized(WindowResized),
    WindowDestroyed(WindowDestroyed),
    CursorMoved(CursorMoved),
    CursorEntered(CursorEntered),
    CursorLeft(CursorLeft),
    ReceivedCharacter(ReceivedCharacter),
    WindowCloseRequested(WindowCloseRequested),
    KeyboardInput(KeyboardInput),
    MouseWheel(MouseWheel),
    MouseButtonInput(MouseButtonInput),
    MouseMotion(MouseMotion),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowCreated {
    pub window: WindowId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowResized {
    pub window: WindowId,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowDestroyed {
    pub window: WindowId,
}

/// A event fired when the cursor moved inside a window.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CursorMoved {
    pub window: WindowId,
    pub position: Vec2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CursorEntered {
    pub window: WindowId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CursorLeft {
    pub window: WindowId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReceivedCharacter {
    pub window: WindowId,
    pub char: char,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowCloseRequested {
    pub window: WindowId,
}

// FIXME: Export a custom type from input crate.
pub use winit::event::VirtualKeyCode;

use crate::windows::WindowId;
