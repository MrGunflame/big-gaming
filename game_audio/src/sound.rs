use std::ops::{Add, AddAssign, Mul, MulAssign};

use crate::effects::Volume;
use crate::sound_data::SoundData;
use crate::spatial::EmitterId;
use crate::track::TrackId;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub left: f32,
    pub right: f32,
}

impl Frame {
    pub const EQUILIBRIUM: Self = Self {
        left: 0.0,
        right: 0.0,
    };

    pub fn from_mono(value: f32) -> Self {
        Self {
            left: value,
            right: value,
        }
    }

    pub fn to_mono(self) -> Self {
        Self::from_mono((self.left + self.right) / 2.0)
    }
}

impl Add for Frame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left + rhs.left,
            right: self.right + rhs.right,
        }
    }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Mul<Volume> for Frame {
    type Output = Self;

    fn mul(self, rhs: Volume) -> Self::Output {
        Self {
            left: self.left * rhs.0,
            right: self.right * rhs.0,
        }
    }
}

impl MulAssign<Volume> for Frame {
    fn mul_assign(&mut self, rhs: Volume) {
        *self = *self * rhs;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(pub(crate) slotmap::DefaultKey);

#[derive(Debug)]
pub(crate) struct PlayingSound {
    pub data: SoundData,
    pub cursor: usize,
    pub destination: Destination,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Destination {
    Track(TrackId),
    Emitter(EmitterId),
}

impl Default for Destination {
    fn default() -> Self {
        Self::Track(TrackId::default())
    }
}

impl From<TrackId> for Destination {
    fn from(id: TrackId) -> Self {
        Self::Track(id)
    }
}

impl From<EmitterId> for Destination {
    fn from(id: EmitterId) -> Self {
        Self::Emitter(id)
    }
}
