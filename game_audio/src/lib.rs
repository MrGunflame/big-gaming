#![deny(unsafe_op_in_unsafe_fn)]

pub mod effects;
pub mod sound_data;
pub mod track;

mod backend;
mod clock;
mod manager;
mod resampler;
mod sound;

pub use manager::AudioManager;

use bevy_app::{App, Plugin};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioManager::new());
    }
}
