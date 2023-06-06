//! UI related systems
#[deny(unsafe_op_in_unsafe_fn)]
pub mod cursor;
// mod interface;
// mod scenes;
// mod systems;

pub mod events;
pub mod render;
pub mod widgets;

pub mod reactive;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoSystemConfig;
use bevy_ecs::system::Query;
use bevy_ecs::world::World;
use cursor::Cursor;
use events::Events;
use reactive::{Document, NodeId, Runtime, Scope};
use render::layout::LayoutTree;
// use cursor::Cursor;
// pub use interface::{Context, InterfaceState, Widget, WidgetFlags};

pub use game_ui_macros::{component, view};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(render::RenderUiPlugin);

        app.insert_resource(Runtime::new());

        // Cursor
        app.insert_resource(Cursor::new());
        app.add_system(cursor::update_cursor_position);

        // Events
        app.add_system(events::update_events_from_layout_tree);
        app.add_system(events::dispatch_cursor_moved_events);
        app.add_system(events::dispatch_mouse_button_input_events);
        app.add_system(events::dispatch_received_character_events);
        app.add_system(events::dispatch_keyboard_input_events);
        app.add_system(events::dispatch_mouse_wheel_events);

        app.add_system(run_effects);
        app.add_system(flush_node_queue.after(run_effects));
    }
}

// fn render_widgets(world: &mut World) {
//     world.resource_scope::<InterfaceState, ()>(|world, mut state| {
//         state.render(world);
//     });
// }

fn run_effects(world: &World, windows: Query<&Document>) {
    for doc in &windows {
        doc.run_effects(world)
    }
}

fn flush_node_queue(mut windows: Query<(&Document, &mut LayoutTree, &mut Events)>) {
    for (doc, mut tree, mut events) in &mut windows {
        doc.flush_node_queue(&mut tree, &mut events);
    }
}
