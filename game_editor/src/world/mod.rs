//! World editiing

use bevy::prelude::Component;

/// An entity that is currently selected.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Selected;

#[derive(Copy, Clone, Debug, Default, Component)]
pub struct EntityOptions {
    pub selected: bool,
    pub hidden: bool,
}
