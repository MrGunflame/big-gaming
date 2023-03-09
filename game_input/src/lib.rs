#![feature(const_trait_impl)]

mod event;

pub mod emulator;
pub mod hotkeys;
pub mod keyboard;
pub mod mouse;

use bevy::prelude::{IntoSystemConfig, IntoSystemSetConfig, Plugin, Resource, SystemSet};
pub use event::*;
use hotkeys::{HotkeyPlugin, HotkeySet};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum InputSet {
    Inputs,
    Hotkeys,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(bevy::input::InputPlugin);
        app.add_event::<keyboard::KeyboardInput>();
        app.add_event::<mouse::MouseMotion>();
        app.add_event::<mouse::MouseButtonInput>();

        app.add_system(keyboard::keyboard_input.in_set(InputSet::Inputs));
        app.add_system(mouse::mouse_motion.in_set(InputSet::Inputs));
        app.add_system(mouse::mouse_buttons.in_set(InputSet::Inputs));

        app.add_plugin(HotkeyPlugin);

        app.insert_resource(CanMouseMove(true));

        app.configure_set(InputSet::Inputs.before(HotkeySet::Reset));
    }
}

/// Should mouse motin events be emitted.
///
/// This will be removed in favor of a consumable event reader in the future.
#[derive(Copy, Clone, Debug, Resource)]
pub struct CanMouseMove(pub bool);
