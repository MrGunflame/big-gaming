//! UI related systems
mod cursor;
// mod interface;
// mod scenes;
// mod systems;

pub mod events;
pub mod render;
pub mod widgets;

use bevy_app::{App, Plugin};
use cursor::Cursor;
// use cursor::Cursor;
// pub use interface::{Context, InterfaceState, Widget, WidgetFlags};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // let state = InterfaceState::new();

        app.add_plugin(render::RenderUiPlugin);

        // Cursor
        app.insert_resource(Cursor::new());
        app.add_system(cursor::update_cursor_position);

        // Events
        app.add_system(events::update_events_from_layout_tree);
        app.add_system(events::dispatch_cursor_moved_events);
    }
}

// fn render_widgets(world: &mut World) {
//     world.resource_scope::<InterfaceState, ()>(|world, mut state| {
//         state.render(world);
//     });
// }
