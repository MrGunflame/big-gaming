#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

pub mod emulator;
pub mod hotkeys;
pub mod keyboard;
pub mod mouse;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::{IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::Resource;
use hotkeys::{HotkeyPlugin, HotkeySet};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum InputSet {
    Inputs,
    Hotkeys,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<keyboard::KeyboardInput>();
        app.add_event::<mouse::MouseMotion>();
        app.add_event::<mouse::MouseButtonInput>();
        app.add_event::<mouse::MouseWheel>();

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    #[inline]
    pub const fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }

    pub const fn is_released(self) -> bool {
        matches!(self, Self::Released)
    }
}
