mod menu;

pub mod crosshair;
pub mod debug;
pub mod health;

use bevy::prelude::Plugin;
use bevy_egui::EguiPlugin;

/// The user interface plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(EguiPlugin)
            .add_system(crosshair::crosshair)
            .add_system(health::health)
            .add_system(debug::debug)
            .add_system(menu::gamemenu::gamemenu);
    }
}
