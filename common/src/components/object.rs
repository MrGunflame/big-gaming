use crate::id::WeakId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A unique identifer for an object.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ObjectId(pub WeakId<u32>);
