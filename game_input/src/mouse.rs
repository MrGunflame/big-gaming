use bevy::prelude::{EventReader, EventWriter, Res, Vec2};

pub use bevy::input::mouse::MouseButton;
pub use bevy::input::ButtonState;

use crate::CanMouseMove;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MouseMotion {
    pub delta: Vec2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ButtonState,
}

pub(super) fn mouse_motion(
    can_move: Res<CanMouseMove>,
    mut reader: EventReader<bevy::input::mouse::MouseMotion>,
    mut writer: EventWriter<MouseMotion>,
) {
    for event in reader.iter() {
        if can_move.0 {
            writer.send(MouseMotion { delta: event.delta });
        }
    }
}

pub(super) fn mouse_buttons(
    mut reader: EventReader<bevy::input::mouse::MouseButtonInput>,
    mut writer: EventWriter<MouseButtonInput>,
) {
    for event in reader.iter() {
        writer.send(MouseButtonInput {
            button: event.button,
            state: event.state,
        });
    }
}
