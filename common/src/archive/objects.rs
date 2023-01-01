//! World objects

use std::borrow::Cow;

use crate::id::StrongId;

use glam::{Quat, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Object<'a> {
    pub id: StrongId<u32>,
    /// A name string only included for later reference. It is not displayed in the game itself.
    pub name: Option<Cow<'a, str>>,
    pub kind: ObjectKind,
    pub segments: Cow<'a, [Segment<'a>]>,
}

/// The type of an [`Object`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ObjectKind {
    Static,
    Dynamic,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Segment<'a> {
    /// The relative translation of the segment.
    pub translation: Vec3,
    /// The relative rotation of the segment.
    pub rotation: Quat,
    /// The handle to the asset.
    pub handle: Cow<'a, str>,
}
