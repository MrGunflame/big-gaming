//! UI related systems
mod cursor;
mod interface;
mod systems;
mod widgets;

use bevy::prelude::{Plugin, Stage};

use cursor::Cursor;
pub use interface::{Context, InterfaceState, Widget, WidgetFlags};
use widgets::{Crosshair, Health};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut state = InterfaceState::new();
        state.push(Health);
        state.push(Crosshair);

        app.add_plugin(bevy_egui::EguiPlugin)
            .insert_resource(state)
            .insert_resource(Cursor::new())
            .add_system(systems::capture_pointer_keys)
            .add_system(systems::death)
            .add_stage("InterfaceStage", InterfaceStage);
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
