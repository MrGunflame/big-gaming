#![feature(const_trait_impl)]

mod event;

pub mod emulator;
pub mod hotkeys;
pub mod keyboard;
pub mod mouse;

use bevy::prelude::{Plugin, Resource};
pub use event::*;
use hotkeys::HotkeyPlugin;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(HotkeyPlugin)
            .add_event::<keyboard::KeyboardInput>()
            .add_event::<mouse::MouseMotion>()
            .add_event::<mouse::MouseButtonInput>()
            .add_system(keyboard::keyboard_input)
            .add_system(mouse::mouse_motion)
            .add_system(mouse::mouse_buttons)
            .insert_resource(CanMouseMove(true));
    }
}

/// Should mouse motin events be emitted.
///
/// This will be removed in favor of a consumable event reader in the future.
#[derive(Copy, Clone, Debug, Resource)]
pub struct CanMouseMove(pub bool);
