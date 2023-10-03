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

    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn from_str_const(s: &str) -> Self {
        let mut bytes = [0; 16];

        let buf = s.as_bytes();
        if buf.len() != 32 {
            panic!("invalid string length");
        }

        let mut index = 0;
        while index < 32 {
            let b = buf[index];

            let mut nibble = match b {
                b'0' => 0,
                b'1' => 1,
                b'2' => 2,
                b'3' => 3,
                b'4' => 4,
                b'5' => 5,
                b'6' => 6,
                b'7' => 7,
                b'8' => 8,
                b'9' => 9,
                b'a' | b'A' => 10,
                b'b' | b'B' => 11,
                b'c' | b'C' => 12,
                b'd' | b'D' => 13,
                b'e' | b'E' => 14,
                b'f' | b'F' => 15,
                _ => panic!("invalid hex digit"),
            };

            // high
            if index % 2 == 0 {
                nibble <<= 4;
            }

            bytes[index / 2] += nibble;
            index += 1;
        }

        Self::from_bytes(bytes)
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
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

    #[test]
    fn module_id_from_str_const() {
        let input = "c2d2a0de054e443ba5e4de7f07262ac7";
        let output = [
            0xc2, 0xd2, 0xa0, 0xde, 0x05, 0x4e, 0x44, 0x3b, 0xa5, 0xe4, 0xde, 0x7f, 0x07, 0x26,
            0x2a, 0xc7,
        ];

        assert_eq!(
            ModuleId::from_str_const(input),
            ModuleId::from_bytes(output)
        );
    }
}
