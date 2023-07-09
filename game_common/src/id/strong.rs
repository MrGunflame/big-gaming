#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[deprecated]
pub struct StrongId<T>(pub T);

impl<T> From<T> for StrongId<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
