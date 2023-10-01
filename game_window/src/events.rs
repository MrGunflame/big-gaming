use game_input::keyboard::{KeyCode, KeyboardInput};
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

pub(crate) fn convert_key_code(key: VirtualKeyCode) -> KeyCode {
    macro_rules! impl_conv {
        ($key:expr, $($id:ident),*$(,)?) => {
            match $key {
                $(
                    VirtualKeyCode::$id => KeyCode::$id,
                )*
            }
        };
    }

    impl_conv!(
        key,
        Key1,
        Key2,
        Key3,
        Key4,
        Key5,
        Key6,
        Key7,
        Key8,
        Key9,
        Key0,
        A,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        Escape,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        F13,
        F14,
        F15,
        F16,
        F17,
        F18,
        F19,
        F20,
        F21,
        F22,
        F23,
        F24,
        Snapshot,
        Scroll,
        Pause,
        Insert,
        Home,
        Delete,
        End,
        PageDown,
        PageUp,
        Left,
        Up,
        Right,
        Down,
        Back,
        Return,
        Space,
        Compose,
        Caret,
        Numlock,
        Numpad0,
        Numpad1,
        Numpad2,
        Numpad3,
        Numpad4,
        Numpad5,
        Numpad6,
        Numpad7,
        Numpad8,
        Numpad9,
        NumpadAdd,
        NumpadDivide,
        NumpadDecimal,
        NumpadComma,
        NumpadEnter,
        NumpadEquals,
        NumpadMultiply,
        NumpadSubtract,
        AbntC1,
        AbntC2,
        Apostrophe,
        Apps,
        Asterisk,
        At,
        Ax,
        Backslash,
        Calculator,
        Capital,
        Colon,
        Comma,
        Convert,
        Equals,
        Grave,
        Kana,
        Kanji,
        LAlt,
        LBracket,
        LControl,
        LShift,
        LWin,
        Mail,
        MediaSelect,
        MediaStop,
        Minus,
        Mute,
        MyComputer,
        NavigateForward,
        NavigateBackward,
        NextTrack,
        NoConvert,
        OEM102,
        Period,
        PlayPause,
        Plus,
        Power,
        PrevTrack,
        RAlt,
        RBracket,
        RControl,
        RShift,
        RWin,
        Semicolon,
        Slash,
        Sleep,
        Stop,
        Sysrq,
        Tab,
        Underline,
        Unlabeled,
        VolumeDown,
        VolumeUp,
        Wake,
        WebBack,
        WebFavorites,
        WebForward,
        WebHome,
        WebRefresh,
        WebSearch,
        WebStop,
        Yen,
        Copy,
        Paste,
        Cut,
    )
}
