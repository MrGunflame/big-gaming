use crate::id::StrongId;
use crate::localization::LocalizedString;
use crate::units::Mass;

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
    /// The equipment slots that this item occupies if it can be equipped.
    ///
    /// `None` if the `Item` can not be equipped.
    pub equipment: Option<Vec<String>>,
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

impl Keywords {
    pub fn contains(&self, keyword: &str) -> bool {
        self.keywords.iter().any(|kv| kv.0.as_ref() == keyword)
    }
}
