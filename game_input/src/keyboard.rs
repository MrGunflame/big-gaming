use crate::ButtonState;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyboardInput {
    pub scan_code: ScanCode,
    pub key_code: Option<KeyCode>,
    pub state: ButtonState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScanCode(pub u32);

macro_rules! impl_keycode {
    ($($key:ident),*,) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub enum KeyCode {
            $(
                $key,
            )*
        }

        impl KeyCode {
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(
                        Self::$key => stringify!($key),
                    )*
                }
            }
        }
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
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
}

impl KeyCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Key1 => "1",
            Self::Key2 => "2",
            Self::Key3 => "3",
            Self::Key4 => "4",
            Self::Key5 => "5",
            Self::Key6 => "6",
            Self::Key7 => "7",
            Self::Key8 => "8",
            Self::Key9 => "9",
            Self::Key0 => "0",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Escape => "Escape",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::F13 => "F13",
        }
    }
}

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// #[repr(transparent)]
// pub struct ScanCode(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Qwertz {}

impl Qwertz {
    /// `^`
    pub const CARET: ScanCode = ScanCode(1);

    pub const KEY_1: ScanCode = ScanCode(2);
    pub const KEY_2: ScanCode = ScanCode(3);
    pub const KEY_3: ScanCode = ScanCode(4);
    pub const KEY_4: ScanCode = ScanCode(5);
    pub const KEY_5: ScanCode = ScanCode(6);
    pub const KEY_6: ScanCode = ScanCode(7);
    pub const KEY_7: ScanCode = ScanCode(8);
    pub const KEY_8: ScanCode = ScanCode(9);
    pub const KEY_9: ScanCode = ScanCode(10);
    pub const KEY_0: ScanCode = ScanCode(11);

    /// `ß`
    pub const KEY_SS: ScanCode = ScanCode(12);

    /// `\``
    pub const KEY_AP: ScanCode = ScanCode(13);

    /// The backspace key.
    pub const BACK: ScanCode = ScanCode(15);

    pub const TAB: ScanCode = ScanCode(16);

    pub const Q: ScanCode = ScanCode(17);
    pub const W: ScanCode = ScanCode(18);
    pub const E: ScanCode = ScanCode(19);
    pub const R: ScanCode = ScanCode(20);
}
