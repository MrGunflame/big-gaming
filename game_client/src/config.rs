use std::fs::File;
use std::io::Read;
use std::path::Path;

use bevy_ecs::system::Resource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Resource)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub timestep: u32,
    pub network: Network,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        Ok(toml::from_str(&buf)?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub interpolation_frames: u32,
}
