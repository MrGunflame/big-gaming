#![deny(unsafe_op_in_unsafe_fn)]

pub mod backend;
pub mod channel;
pub mod effects;
pub mod sound;
pub mod sound_data;
pub mod spatial;
pub mod track;

mod clock;
mod manager;
mod resampler;

use backend::DefaultBackend;
pub use manager::AudioManager;

use bevy_app::{App, Plugin};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let backend = DefaultBackend::new();
        app.insert_resource(AudioManager::new(backend));
    }
}
