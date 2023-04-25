use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use bytemuck::{Pod, Zeroable};
use hex::FromHexError;
use thiserror::Error;
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

#[derive(Copy, Clone, Debug, Error)]
pub enum ParseModuleIdError {
    #[error("invalid string length: {0}, expected: 16")]
    Length(usize),
    #[error("invalid hex: {0}")]
    InvalidHex(FromHexError),
}

impl FromStr for ModuleId {
    type Err = ParseModuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let buf = hex::decode(s).map_err(ParseModuleIdError::InvalidHex)?;

        if buf.len() != 16 {
            return Err(ParseModuleIdError::Length(buf.len()));
        }

        Ok(Self(buf[0..16].try_into().unwrap()))
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

#[cfg(test)]
mod tests {
    use super::ModuleId;

    #[test]
    fn module_id_from_str() {
        let input = "c2d2a0de054e443ba5e4de7f07262ac7";
        let output = [
            0xc2, 0xd2, 0xa0, 0xde, 0x05, 0x4e, 0x44, 0x3b, 0xa5, 0xe4, 0xde, 0x7f, 0x07, 0x26,
            0x2a, 0xc7,
        ];

        assert_eq!(
            input.parse::<ModuleId>().unwrap(),
            ModuleId::from_bytes(output)
        );
    }
}
