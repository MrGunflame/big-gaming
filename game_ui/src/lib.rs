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
use reactive::{init_runtime, ReactiveRoot, Runtime, Scope};
use render::layout::LayoutTree;
// use cursor::Cursor;
// pub use interface::{Context, InterfaceState, Widget, WidgetFlags};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        init_runtime();
        // let state = InterfaceState::new();

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

fn drive_reactive_runtime(mut windows: Query<(&mut LayoutTree, &mut ReactiveRoot)>) {
    for (tree, mut root) in &mut windows {
        if root.is_first_run {
            root.is_first_run = false;
            // (root.f)();
        }
    }
}
