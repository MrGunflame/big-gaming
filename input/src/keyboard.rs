use bevy::input::ButtonState;
use bevy::prelude::{EventReader, EventWriter, Input, ResMut};

pub use bevy::prelude::{KeyCode, ScanCode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyboardInput {
    pub scan_code: ScanCode,
    pub key_code: Option<KeyCode>,
    pub state: ButtonState,
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

pub fn keyboard_input(
    mut reader: EventReader<bevy::input::keyboard::KeyboardInput>,
    mut writer: EventWriter<KeyboardInput>,
) {
    for event in reader.iter() {
        writer.send(KeyboardInput {
            scan_code: ScanCode(event.scan_code),
            key_code: event.key_code,
            state: event.state,
        });
    }
}
