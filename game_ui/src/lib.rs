//! UI related systems
mod cursor;
mod interface;
mod scenes;
mod systems;

pub mod widgets;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::{Plugin, Stage};

use cursor::Cursor;
pub use interface::{Context, InterfaceState, Widget, WidgetFlags};
use widgets::{Crosshair, Health, Weapon};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut state = InterfaceState::new();

        app.add_plugin(bevy_egui::EguiPlugin)
            .add_plugin(FrameTimeDiagnosticsPlugin)
            .insert_resource(state)
            .insert_resource(Cursor::new())
            .add_startup_system(widgets::register_hotkeys)
            .add_system(systems::capture_pointer_keys)
            .add_system(systems::death)
            .add_stage("InterfaceStage", InterfaceStage);

        #[cfg(not(feature = "editor"))]
        app.add_system(systems::scene_transition);

        widgets::register_hotkey_systems(app);
    }
}

struct InterfaceStage;

impl Stage for InterfaceStage {
    fn run(&mut self, world: &mut bevy::prelude::World) {
        world.resource_scope::<InterfaceState, ()>(|world, mut state| {
            state.render(world);
        });
    }
}
