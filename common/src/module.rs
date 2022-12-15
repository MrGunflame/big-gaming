use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub version: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId([u8; 16]);

impl ModuleId {}

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
