use std::fs::File;
use std::io::{self, Read, Write};
use std::num::NonZeroU32;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml_edit::{DocumentMut, Formatted, Item, Key, Table, Value};

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
    /// Loads the config from the file with the given `path`.
    ///
    /// # Errors
    ///
    /// Returns an [`ConfigError`] if `path` is readable as a file or contains invalid data.
    pub fn from_file<P>(path: P) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path).map_err(ConfigError::Io)?;

        let mut buf = String::new();
        file.read_to_string(&mut buf).map_err(ConfigError::Io)?;

        toml::from_str(&buf).map_err(ConfigError::Toml)
    }

    /// Creates a new config file at the given `path`.
    ///
    /// # Errors
    ///
    /// Returns an [`ConfigError`] if creating the file fails.
    pub fn create_default_config<P>(path: P) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
    {
        let mut file = File::create_new(path).map_err(ConfigError::Io)?;

        let config = Config::default();

        let mut document = DocumentMut::new();
        config.write_config_values(document.as_table_mut());
        let s = document.to_string();
        file.write_all(s.as_bytes()).map_err(ConfigError::Io)?;

        Ok(config)
    }

    fn write_config_values(&self, table: &mut Table) {
        write_field(table, "timestep", "Timestep", self.timestep);

        let mut network = Table::new();
        self.network.write_config_values(&mut network);
        table.insert("network", Item::Table(network));

        let mut graphics = Table::new();
        self.graphics.write_config_values(&mut graphics);
        table.insert("graphics", Item::Table(graphics));
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

impl Network {
    fn write_config_values(&self, table: &mut Table) {
        write_field(
            table,
            "interpolation_frames",
            "Interpolation frames",
            self.interpolation_frames,
        );
        write_field(
            table,
            "prediction",
            "Whether client-side prediction is enabled.",
            self.prediction,
        );
    }
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
    fps_limit: u32,
}

impl Graphics {
    /// Returns the configured `fps_limit` value.
    pub fn fps_limit(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.fps_limit)
    }

    fn write_config_values(&self, table: &mut Table) {
        write_field(
            table,
            "fps_limit",
            "FPS limit of the renderer. A value of `0` means unlimited.",
            self.fps_limit,
        );
    }
}

impl Default for Graphics {
    fn default() -> Self {
        Self { fps_limit: 0 }
    }
}

fn write_field<T>(
    table: &mut Table,
    name: &'static str,
    description: &'static str,
    default_value: T,
) where
    T: ConfigValue,
{
    let key = Key::new(name);
    let default_value = default_value.into_value();

    let default_value_t = match &default_value {
        Value::String(val) => Some(val.value().clone()),
        Value::Integer(val) => Some(val.to_string()),
        Value::Float(val) => Some(val.to_string()),
        Value::Boolean(val) => Some(val.to_string()),
        _ => None,
    };

    let prefix = if let Some(default_value_t) = default_value_t {
        format!("# {}\n# Default value: {}\n", description, default_value_t)
    } else {
        format!("# {}\n", description)
    };

    table.insert_formatted(&key, Item::Value(default_value));
    table
        .key_mut(&key)
        .unwrap()
        .leaf_decor_mut()
        .set_prefix(prefix);
}

trait ConfigValue {
    fn into_value(self) -> Value;
}

macro_rules! impl_int {
    ($($t:ty),*) => {
        $(
            impl ConfigValue for $t {
                fn into_value(self) -> Value {
                    Value::Integer(Formatted::new(self.into()))
                }
            }
        )*
    };
}

impl_int! { u8, u16, u32, i8, i16, i32, i64 }

impl ConfigValue for bool {
    fn into_value(self) -> Value {
        Value::Boolean(Formatted::new(self))
    }
}
