use crate::localization::LocalizedString;

#[derive(Clone, Debug)]
pub struct Item {
    pub id: u64,
    pub name: LocalizedString,
    pub mass: Mass,
    // TODO: These should probably not be hardcoded onto an item.
}

/// The mass/weight of an [`Item`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mass(u32);

impl Mass {
    pub const fn grams(g: u32) -> Self {
        Self(g)
    }

    pub const fn kilograms(kg: u32) -> Self {
        Self(kg * 1000)
    }

    pub const fn as_grams(self) -> u32 {
        self.0
    }

    pub fn as_kilograms_f32(self) -> f32 {
        self.0 as f32 / 1000.0
    }
}
