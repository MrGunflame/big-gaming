use crate::id::StrongId;
use crate::localization::LocalizedString;
use crate::types::Mass;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Item {
    pub id: StrongId<u32>,
    pub name: LocalizedString,
    pub mass: Mass,
    #[serde(default)]
    pub keywords: Keywords,
    // TODO: These should probably not be hardcoded onto an item.
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Keyword(Box<str>);

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Keywords {
    keywords: Vec<Keyword>,
}
