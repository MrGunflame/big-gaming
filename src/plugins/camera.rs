use bevy::prelude::Plugin;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(crate::systems::input::keyboard_input)
            .add_system(crate::systems::input::mouse_input)
            .add_system(crate::systems::input::mouse_scroll)
            .add_system(crate::systems::input::transform_system)
            .add_system(crate::systems::input::sync_player_camera)
            .add_system(crate::systems::input::mouse_button_input);
    }
}
