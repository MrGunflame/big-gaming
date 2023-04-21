use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub timestep: u32,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let s = std::str::from_utf8(&buf)?;

        Ok(toml::from_str(s)?)
    }
}
