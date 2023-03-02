//! World editiing
//!
pub mod axes;

pub const COLOR_X: Color = Color::rgb(1.0, 0.0, 0.0);
pub const COLOR_Y: Color = Color::rgb(0.0, 1.0, 0.0);
pub const COLOR_Z: Color = Color::rgb(0.0, 0.0, 1.0);

use bevy::prelude::{Color, Component};

/// An entity that is currently selected.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Selected;

#[derive(Copy, Clone, Debug, Default, Component)]
pub struct EntityOptions {
    pub selected: bool,
    pub hidden: bool,
}
