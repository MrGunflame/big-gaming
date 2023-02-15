//! Debugging related plugins

use bevy::prelude::MaterialPlugin;

use self::cell::CellFrameMaterial;
pub mod actor_trace;
mod cell;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DebugPlugin;

impl bevy::prelude::Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(MaterialPlugin::<CellFrameMaterial>::default())
            .add_system(cell::render_cell_borders);
    }
}
