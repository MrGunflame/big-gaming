mod event;

pub mod hotkeys;
pub mod keyboard;

use bevy::prelude::Plugin;
pub use event::*;
use hotkeys::HotkeyPlugin;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(HotkeyPlugin)
            .add_event::<keyboard::KeyboardInput>()
            .add_system(keyboard::keyboard_input);
    }
}
