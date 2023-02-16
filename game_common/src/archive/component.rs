use crate::id::StrongId;
use crate::localization::LocalizedString;
use crate::units::Mass;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Component {
    pub id: StrongId<u32>,
    pub name: LocalizedString,
    /// Standalone mass of the `Component`.
    pub mass: Mass,
}
