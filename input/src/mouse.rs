use bevy::prelude::{EventReader, EventWriter, Res, Vec2};

use crate::CanMouseMove;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MouseMotion {
    pub delta: Vec2,
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
