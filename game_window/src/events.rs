use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::{MouseButtonInput, MouseMotion, MouseWheel};
use glam::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub enum WindowEvent {
    WindowCreated(WindowCreated),
    WindowResized(WindowResized),
    WindowDestroyed(WindowDestroyed),
    CursorMoved(CursorMoved),
    CursorEntered(CursorEntered),
    CursorLeft(CursorLeft),
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
pub struct WindowCloseRequested {
    pub window: WindowId,
}

use crate::windows::WindowId;

pub(crate) fn convert_key_code(key: winit::keyboard::KeyCode) -> Option<KeyCode> {
    match key {
        winit::keyboard::KeyCode::Digit1 => Some(KeyCode::Key1),
        winit::keyboard::KeyCode::Digit2 => Some(KeyCode::Key2),
        winit::keyboard::KeyCode::Digit3 => Some(KeyCode::Key3),
        winit::keyboard::KeyCode::Digit4 => Some(KeyCode::Key4),
        winit::keyboard::KeyCode::Digit5 => Some(KeyCode::Key5),
        winit::keyboard::KeyCode::Digit6 => Some(KeyCode::Key6),
        winit::keyboard::KeyCode::Digit7 => Some(KeyCode::Key7),
        winit::keyboard::KeyCode::Digit8 => Some(KeyCode::Key8),
        winit::keyboard::KeyCode::Digit9 => Some(KeyCode::Key9),
        winit::keyboard::KeyCode::Digit0 => Some(KeyCode::Key0),
        winit::keyboard::KeyCode::KeyA => Some(KeyCode::A),
        winit::keyboard::KeyCode::KeyB => Some(KeyCode::B),
        winit::keyboard::KeyCode::KeyC => Some(KeyCode::C),
        winit::keyboard::KeyCode::KeyD => Some(KeyCode::D),
        winit::keyboard::KeyCode::KeyE => Some(KeyCode::E),
        winit::keyboard::KeyCode::KeyF => Some(KeyCode::F),
        winit::keyboard::KeyCode::KeyG => Some(KeyCode::G),
        winit::keyboard::KeyCode::KeyH => Some(KeyCode::H),
        winit::keyboard::KeyCode::KeyI => Some(KeyCode::I),
        winit::keyboard::KeyCode::KeyJ => Some(KeyCode::J),
        winit::keyboard::KeyCode::KeyK => Some(KeyCode::K),
        winit::keyboard::KeyCode::KeyL => Some(KeyCode::L),
        winit::keyboard::KeyCode::KeyM => Some(KeyCode::M),
        winit::keyboard::KeyCode::KeyN => Some(KeyCode::N),
        winit::keyboard::KeyCode::KeyO => Some(KeyCode::O),
        winit::keyboard::KeyCode::KeyP => Some(KeyCode::P),
        winit::keyboard::KeyCode::KeyQ => Some(KeyCode::Q),
        winit::keyboard::KeyCode::KeyR => Some(KeyCode::R),
        winit::keyboard::KeyCode::KeyS => Some(KeyCode::S),
        winit::keyboard::KeyCode::KeyT => Some(KeyCode::T),
        winit::keyboard::KeyCode::KeyU => Some(KeyCode::U),
        winit::keyboard::KeyCode::KeyV => Some(KeyCode::V),
        winit::keyboard::KeyCode::KeyW => Some(KeyCode::W),
        winit::keyboard::KeyCode::KeyX => Some(KeyCode::X),
        winit::keyboard::KeyCode::KeyY => Some(KeyCode::Y),
        winit::keyboard::KeyCode::KeyZ => Some(KeyCode::Z),
        winit::keyboard::KeyCode::Escape => Some(KeyCode::Escape),
        winit::keyboard::KeyCode::F1 => Some(KeyCode::F1),
        winit::keyboard::KeyCode::F2 => Some(KeyCode::F2),
        winit::keyboard::KeyCode::F3 => Some(KeyCode::F3),
        winit::keyboard::KeyCode::F4 => Some(KeyCode::F4),
        winit::keyboard::KeyCode::F5 => Some(KeyCode::F5),
        winit::keyboard::KeyCode::F6 => Some(KeyCode::F6),
        winit::keyboard::KeyCode::F7 => Some(KeyCode::F7),
        winit::keyboard::KeyCode::F8 => Some(KeyCode::F8),
        winit::keyboard::KeyCode::F9 => Some(KeyCode::F9),
        winit::keyboard::KeyCode::F10 => Some(KeyCode::F10),
        winit::keyboard::KeyCode::F11 => Some(KeyCode::F11),
        winit::keyboard::KeyCode::F12 => Some(KeyCode::F12),
        winit::keyboard::KeyCode::F13 => Some(KeyCode::F13),
        winit::keyboard::KeyCode::F14 => Some(KeyCode::F14),
        winit::keyboard::KeyCode::F15 => Some(KeyCode::F15),
        winit::keyboard::KeyCode::F16 => Some(KeyCode::F16),
        winit::keyboard::KeyCode::F17 => Some(KeyCode::F17),
        winit::keyboard::KeyCode::F18 => Some(KeyCode::F18),
        winit::keyboard::KeyCode::F19 => Some(KeyCode::F19),
        winit::keyboard::KeyCode::F20 => Some(KeyCode::F20),
        winit::keyboard::KeyCode::F21 => Some(KeyCode::F21),
        winit::keyboard::KeyCode::F22 => Some(KeyCode::F22),
        winit::keyboard::KeyCode::F23 => Some(KeyCode::F23),
        winit::keyboard::KeyCode::F24 => Some(KeyCode::F24),
        winit::keyboard::KeyCode::NumLock => Some(KeyCode::Numlock),
        winit::keyboard::KeyCode::Numpad0 => Some(KeyCode::Numpad0),
        winit::keyboard::KeyCode::Numpad1 => Some(KeyCode::Numpad1),
        winit::keyboard::KeyCode::Numpad2 => Some(KeyCode::Numpad2),
        winit::keyboard::KeyCode::Numpad3 => Some(KeyCode::Numpad3),
        winit::keyboard::KeyCode::Numpad4 => Some(KeyCode::Numpad4),
        winit::keyboard::KeyCode::Numpad5 => Some(KeyCode::Numpad5),
        winit::keyboard::KeyCode::Numpad6 => Some(KeyCode::Numpad6),
        winit::keyboard::KeyCode::Numpad7 => Some(KeyCode::Numpad7),
        winit::keyboard::KeyCode::Numpad8 => Some(KeyCode::Numpad8),
        winit::keyboard::KeyCode::Numpad9 => Some(KeyCode::Numpad9),
        winit::keyboard::KeyCode::NumpadAdd => Some(KeyCode::Numpad9),
        winit::keyboard::KeyCode::NumpadDivide => Some(KeyCode::NumpadDivide),
        winit::keyboard::KeyCode::NumpadComma => Some(KeyCode::NumpadComma),
        winit::keyboard::KeyCode::NumpadDecimal => Some(KeyCode::NumpadDecimal),
        winit::keyboard::KeyCode::NumpadEnter => Some(KeyCode::NumpadEnter),
        winit::keyboard::KeyCode::NumpadSubtract => Some(KeyCode::NumpadSubtract),
        winit::keyboard::KeyCode::NumpadMultiply => Some(KeyCode::NumpadMultiply),
        winit::keyboard::KeyCode::NumpadEqual => Some(KeyCode::NumpadEquals),
        winit::keyboard::KeyCode::ShiftLeft => Some(KeyCode::LShift),
        winit::keyboard::KeyCode::ShiftRight => Some(KeyCode::RShift),
        winit::keyboard::KeyCode::ControlLeft => Some(KeyCode::LControl),
        winit::keyboard::KeyCode::ControlRight => Some(KeyCode::RControl),
        winit::keyboard::KeyCode::AltLeft => Some(KeyCode::LAlt),
        winit::keyboard::KeyCode::AltRight => Some(KeyCode::RAlt),
        winit::keyboard::KeyCode::Tab => Some(KeyCode::Tab),
        winit::keyboard::KeyCode::Space => Some(KeyCode::Space),
        winit::keyboard::KeyCode::BracketLeft => Some(KeyCode::LBracket),
        winit::keyboard::KeyCode::BracketRight => Some(KeyCode::RBracket),
        winit::keyboard::KeyCode::Semicolon => Some(KeyCode::Semicolon),
        winit::keyboard::KeyCode::Minus => Some(KeyCode::Minus),
        winit::keyboard::KeyCode::Equal => Some(KeyCode::Equals),
        winit::keyboard::KeyCode::Backslash => Some(KeyCode::Backslash),
        winit::keyboard::KeyCode::Slash => Some(KeyCode::Slash),
        winit::keyboard::KeyCode::SuperLeft => Some(KeyCode::LSuper),
        winit::keyboard::KeyCode::SuperRight => Some(KeyCode::RSuper),
        winit::keyboard::KeyCode::ArrowLeft => Some(KeyCode::Left),
        winit::keyboard::KeyCode::ArrowRight => Some(KeyCode::Right),
        winit::keyboard::KeyCode::ArrowDown => Some(KeyCode::Down),
        winit::keyboard::KeyCode::ArrowUp => Some(KeyCode::Up),
        _ => None,
    }
}
