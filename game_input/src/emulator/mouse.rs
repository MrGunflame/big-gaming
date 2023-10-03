use std::collections::VecDeque;

use glam::Vec2;

use crate::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use crate::ButtonState;

#[derive(Clone, Debug, Default)]
pub struct MouseEmulator {
    buttons: VecDeque<MouseButtonInput>,
    motions: VecDeque<MouseMotion>,
}

impl MouseEmulator {
    pub fn new() -> Self {
        Self {
            buttons: VecDeque::new(),
            motions: VecDeque::new(),
        }
    }

    pub fn press(&mut self, button: MouseButton) {
        self.buttons.push_back(MouseButtonInput {
            button,
            state: ButtonState::Pressed,
        });
    }

    pub fn release(&mut self, button: MouseButton) {
        self.buttons.push_back(MouseButtonInput {
            button,
            state: ButtonState::Released,
        });
    }

    pub fn motion(&mut self, delta: Vec2) {
        self.motions.push_back(MouseMotion { delta });
    }

    pub fn pop_button(&mut self) -> Option<MouseButtonInput> {
        self.buttons.pop_front()
    }

    pub fn pop_motion(&mut self) -> Option<MouseMotion> {
        self.motions.pop_front()
    }
}
