#![deny(unsafe_op_in_unsafe_fn)]

mod backend;
mod manager;
mod sound;

use bevy_app::{App, Plugin};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{default_host, Host};
use manager::AudioManager;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioManager::new());
    }
}
