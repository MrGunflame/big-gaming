use serde::{Deserialize, Serialize};

/// The health value of an actor.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Health(u32);

impl Health {
    #[inline]
    pub const fn new(val: u32) -> Self {
        Self(val)
    }
}
