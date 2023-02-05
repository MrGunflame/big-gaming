use std::collections::VecDeque;

use bevy::prelude::{EventWriter, Plugin, ResMut, Resource};

use crate::keyboard::{ButtonState, KeyCode, KeyboardInput, ScanCode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyboardEmulatorPlugin;

impl Plugin for KeyboardEmulatorPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(KeyboardEmulator::new())
            .add_system(emulate_keyboard);
    }
}

/// An emulator for a keyboard device.
#[derive(Clone, Debug, Resource)]
pub struct KeyboardEmulator {
    queue: VecDeque<KeyboardInput>,
}

impl KeyboardEmulator {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn press(&mut self, key_code: KeyCode) {
        self.queue.push_back(KeyboardInput {
            scan_code: ScanCode(0),
            key_code: Some(key_code),
            state: ButtonState::Pressed,
        });
    }

    pub fn release(&mut self, key_code: KeyCode) {
        self.queue.push_back(KeyboardInput {
            scan_code: ScanCode(0),
            key_code: Some(key_code),
            state: ButtonState::Released,
        });
    }

    fn pop(&mut self) -> Option<KeyboardInput> {
        self.queue.pop_front()
    }
}

fn emulate_keyboard(
    mut emulator: ResMut<KeyboardEmulator>,
    mut writer: EventWriter<KeyboardInput>,
) {
    while let Some(event) = emulator.pop() {
        writer.send(event);
    }
}
