use std::collections::VecDeque;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::EventWriter;
use bevy_ecs::system::{ResMut, Resource};
use glam::Vec2;

use crate::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use crate::ButtonState;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MouseEmulatorPlugin;

impl Plugin for MouseEmulatorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MouseEmulator::new())
            .add_system(emulate_mouse_buttons)
            .add_system(emulate_mouse_motions);
    }
}

#[derive(Clone, Debug, Resource)]
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

    fn pop_button(&mut self) -> Option<MouseButtonInput> {
        self.buttons.pop_front()
    }

    fn pop_motion(&mut self) -> Option<MouseMotion> {
        self.motions.pop_front()
    }
}

fn emulate_mouse_buttons(
    mut emulator: ResMut<MouseEmulator>,
    mut writer: EventWriter<MouseButtonInput>,
) {
    while let Some(event) = emulator.pop_button() {
        writer.send(event);
    }
}

fn emulate_mouse_motions(
    mut emulator: ResMut<MouseEmulator>,
    mut writer: EventWriter<MouseMotion>,
) {
    while let Some(event) = emulator.pop_motion() {
        writer.send(event);
    }
}
