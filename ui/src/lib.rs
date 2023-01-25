//! UI related systems
mod interface;
mod widgets;

use bevy::prelude::{Plugin, Stage};

pub use interface::{InterfaceState, Widget, WidgetFlags};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(InterfaceState::new())
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
