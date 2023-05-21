//! UI related systems
pub mod cursor;
// mod interface;
// mod scenes;
// mod systems;

pub mod events;
pub mod render;
pub mod widgets;

pub mod reactive;

use bevy_app::{App, Plugin};
use bevy_ecs::system::Query;
use bevy_ecs::world::World;
use cursor::Cursor;
use reactive::{Document, NodeId, Scope};
use render::layout::LayoutTree;
// use cursor::Cursor;
// pub use interface::{Context, InterfaceState, Widget, WidgetFlags};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(render::RenderUiPlugin);

        // Cursor
        app.insert_resource(Cursor::new());
        app.add_system(cursor::update_cursor_position);

        // Events
        app.add_system(events::update_events_from_layout_tree);
        app.add_system(events::dispatch_cursor_moved_events);
        app.add_system(events::dispatch_mouse_button_input_events);

        app.add_system(drive_reactive_runtime);
    }
}

// fn render_widgets(world: &mut World) {
//     world.resource_scope::<InterfaceState, ()>(|world, mut state| {
//         state.render(world);
//     });
// }

fn drive_reactive_runtime(world: &World, windows: Query<&Document>) {
    for doc in &windows {
        doc.drive(world);
    }
}
