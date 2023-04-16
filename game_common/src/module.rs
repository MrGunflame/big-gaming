use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use bytemuck::{Pod, Zeroable};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
}

impl Module {
    // FIXME: Change this to constant if possible.
    pub fn core() -> Self {
        Self {
            id: ModuleId::CORE,
            name: String::from("core"),
            version: Version,
            dependencies: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct ModuleId([u8; 16]);

impl ModuleId {
    pub const CORE: Self = Self([0; 16]);

    /// Creates and returns a new random `ModuleId`.
    pub fn random() -> Self {
        let uuid = Uuid::new_v4();
        Self(uuid.into_bytes())
    }

    pub fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl FromStr for ModuleId {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let buf = hex::decode(s)?;

        // TODO: Error handling
        Ok(Self(buf[0..15].try_into().unwrap()))
    }
}

#[derive(Clone, Debug)]
pub struct Dependency {
    pub id: ModuleId,
    pub name: Option<String>,
    //TODO
    pub version: Version,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version;
