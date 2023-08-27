use glam::Vec3;
use slotmap::{DefaultKey, SlotMap};

use crate::track::TrackId;

#[derive(Copy, Clone, Debug)]
pub struct Listener {
    pub track: TrackId,
    pub position: Vec3,
    pub left_ear: Vec3,
    pub right_ear: Vec3,
}

#[derive(Copy, Clone, Debug)]
pub struct Emitter {
    pub position: Vec3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmitterId(pub(crate) DefaultKey);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(pub(crate) DefaultKey);
