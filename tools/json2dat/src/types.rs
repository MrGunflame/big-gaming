use std::fmt::{self, Formatter};

use game_common::module::ModuleId;
use game_common::record::RecordId;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Root {
    pub id: JsonModuleId,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: Vec<JsonDependency>,
    #[serde(default)]
    pub records: Records,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Records {
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JsonModuleId(pub ModuleId);

impl Serialize for JsonModuleId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for JsonModuleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Vis;

        impl<'de> Visitor<'de> for Vis {
            type Value = JsonModuleId;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("a hex-encoded module id")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.parse() {
                    Ok(val) => Ok(JsonModuleId(val)),
                    Err(err) => Err(E::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(Vis)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JsonRecordId(pub RecordId);

impl Serialize for JsonRecordId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for JsonRecordId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Vis;

        impl<'de> Visitor<'de> for Vis {
            type Value = JsonRecordId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex-encoded record id")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.parse() {
                    Ok(val) => Ok(JsonRecordId(val)),
                    Err(err) => Err(E::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(Vis)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonDependency {
    pub id: JsonModuleId,
    pub name: Option<String>,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub id: JsonRecordId,
    pub name: String,
    pub description: String,
    pub scripts: Vec<String>,
}
