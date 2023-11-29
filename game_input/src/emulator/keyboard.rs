use std::collections::VecDeque;

use crate::keyboard::{KeyCode, KeyboardInput, ScanCode};
use crate::ButtonState;

/// An emulator for a keyboard device.
#[derive(Clone, Debug, Default)]
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
            text: None,
        });
    }

    pub fn release(&mut self, key_code: KeyCode) {
        self.queue.push_back(KeyboardInput {
            scan_code: ScanCode(0),
            key_code: Some(key_code),
            state: ButtonState::Released,
            text: None,
        });
    }

    pub fn pop(&mut self) -> Option<KeyboardInput> {
        self.queue.pop_front()
    }
}
