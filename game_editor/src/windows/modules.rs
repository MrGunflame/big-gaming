//! Module selectors

use bevy::prelude::Component;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleWindowPlugin;

impl bevy::prelude::Plugin for ModuleWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct ModuleWindow;
