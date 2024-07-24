use std::fs::File;
use std::io::{self, Read};
use std::num::NonZeroU32;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(io::Error),
    #[error(transparent)]
    Toml(toml::de::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub timestep: u32,
    pub network: Network,
    pub graphics: Graphics,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path).map_err(ConfigError::Io)?;

        let mut buf = String::new();
        file.read_to_string(&mut buf).map_err(ConfigError::Io)?;

        toml::from_str(&buf).map_err(ConfigError::Toml)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timestep: 60,
            network: Network::default(),
            graphics: Graphics::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub interpolation_frames: u16,
    /// Should client-side prediction be enabled?
    ///
    /// Defaults to `true`.
    pub prediction: bool,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            interpolation_frames: 6,
            prediction: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graphics {
    pub fps_limit: Option<NonZeroU32>,
}

impl Default for Graphics {
    fn default() -> Self {
        Self { fps_limit: None }
    }
}
