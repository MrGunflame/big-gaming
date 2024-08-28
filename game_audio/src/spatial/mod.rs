use game_common::collections::arena::Key;
use glam::{Quat, Vec3};

use crate::sound::Frame;
use crate::track::TrackId;

const EAR_DISTANCE: f32 = 0.1;
const MIN_VOLUME: f32 = 0.3;

#[derive(Copy, Clone, Debug)]
pub struct Listener {
    pub track: TrackId,
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Listener {
    fn ear_translations(&self) -> (Vec3, Vec3) {
        // With our coordinate space, left is -X, right is X.
        let left = self.translation + self.rotation * (-Vec3::X * EAR_DISTANCE);
        let right = self.translation + self.rotation * (Vec3::X * EAR_DISTANCE);
        (left, right)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Emitter {
    pub translation: Vec3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmitterId(pub(crate) Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(pub(crate) Key);

pub(crate) fn process(listener: &Listener, emitter: &Emitter, frame: Frame) -> Frame {
    let mut output = frame.to_mono();

    let (left_pos, right_pos) = listener.ear_translations();
    let left_dir = listener.rotation * -Vec3::X;
    let right_dir = listener.rotation * Vec3::X;
    debug_assert_ne!(left_pos, right_pos);

    let distance = f32::max(1.0, (emitter.translation - listener.translation).length());
    output.left = output.left / distance;
    output.right = output.right / distance;

    let emitter_left_dir = (emitter.translation - left_pos).normalize_or_zero();
    let emitter_right_dir = (emitter.translation - right_pos).normalize_or_zero();

    let left_vol = (left_dir.dot(emitter_left_dir) + 1.0) / 2.0;
    let right_vol = (right_dir.dot(emitter_right_dir) + 1.0) / 2.0;

    output.left *= MIN_VOLUME + (1.0 - MIN_VOLUME) * left_vol;
    output.right *= MIN_VOLUME + (1.0 - MIN_VOLUME) * right_vol;

    output
}
