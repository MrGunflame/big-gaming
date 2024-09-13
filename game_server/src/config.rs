use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::str::Utf8Error;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub timestep: u32,
    pub player_streaming_source_distance: u32,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self, LoadConfigError>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path).map_err(LoadConfigError::Io)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(LoadConfigError::Io)?;

        let s = std::str::from_utf8(&buf).map_err(LoadConfigError::Utf8)?;

        toml::from_str(s).map_err(LoadConfigError::Toml)
    }

    pub fn create_default_config<P>(path: P) -> Result<Self, LoadConfigError>
    where
        P: AsRef<Path>,
    {
        let mut file = File::create_new(path).map_err(LoadConfigError::Io)?;

        let config = Self::default();

        let s = toml::to_string(&config).unwrap();
        file.write_all(s.as_bytes()).map_err(LoadConfigError::Io)?;

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timestep: 60,
            player_streaming_source_distance: 2,
        }
    }
}

#[derive(Debug, Error)]
pub enum LoadConfigError {
    #[error(transparent)]
    Io(io::Error),
    #[error(transparent)]
    Utf8(Utf8Error),
    #[error(transparent)]
    Toml(toml::de::Error),
}
