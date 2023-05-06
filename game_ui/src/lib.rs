//! UI related systems
// mod cursor;
// mod interface;
// mod scenes;
// mod systems;

// pub mod widgets;

pub mod render;

use bevy_app::{App, Plugin};
// use cursor::Cursor;
// pub use interface::{Context, InterfaceState, Widget, WidgetFlags};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // let state = InterfaceState::new();

        app.add_plugin(render::RenderUiPlugin);

        // app.add_plugin(bevy_egui::EguiPlugin)
        //     .add_plugin(FrameTimeDiagnosticsPlugin)
        //     .insert_resource(state)
        //     .insert_resource(Cursor::new())
        //     .add_startup_system(widgets::register_hotkeys)
        //     .add_system(systems::capture_pointer_keys)
        //     .add_system(systems::death)
        //     .add_system(render_widgets);

        // #[cfg(not(feature = "editor"))]
        // app.add_system(systems::scene_transition);

        // widgets::register_hotkey_systems(app);
    }
}

// fn render_widgets(world: &mut World) {
//     world.resource_scope::<InterfaceState, ()>(|world, mut state| {
//         state.render(world);
//     });
// }
