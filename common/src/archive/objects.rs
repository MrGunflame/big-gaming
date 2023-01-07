//! World objects

use crate::id::StrongId;

use glam::{Quat, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Object {
    pub id: StrongId<u32>,
    /// A name string only included for later reference. It is not displayed in the game itself.
    pub name: Option<String>,
    pub kind: ObjectKind,
    pub segments: Option<Box<[Segment]>>,
    pub handle: Option<String>,
    pub collider: Collider,
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
pub struct Segment {
    /// UNIMPLEMENTED
    pub id: (),
    /// The relative translation of the segment.
    pub translation: Vec3,
    /// The relative rotation of the segment.
    pub rotation: Quat,
    /// The scale applied to the segment.
    pub scale: Vec3,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Collider {
    Cuboid(Vec3),
}
