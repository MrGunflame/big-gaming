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
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub kind: ObjectKind,
    pub handle: Option<String>,
}

/// The type of an [`Object`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ObjectKind {
    Static,
    Dynamic,
}
