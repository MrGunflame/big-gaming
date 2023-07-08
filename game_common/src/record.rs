use std::fmt::{self, Display, Formatter, LowerHex};
use std::str::FromStr;

use bytemuck::{Pod, Zeroable};
use hex::FromHexError;
use thiserror::Error;

use crate::module::{ModuleId, ParseModuleIdError};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct RecordId(pub u32);

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl FromStr for RecordId {
    type Err = ParseRecordIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(ParseRecordIdError::InvalidHex)?;

        if bytes.len() == 0 || bytes.len() > 4 {
            return Err(ParseRecordIdError::Length(bytes.len()));
        }

        let mut bits = 0;
        match bytes.len() {
            1 => {
                bits += bytes[0] as u32;
            }
            2 => {
                bits += (bytes[0] as u32) << 8;
                bits += bytes[1] as u32;
            }
            3 => {
                bits += (bytes[0] as u32) << 16;
                bits += (bytes[1] as u32) << 8;
                bits += bytes[2] as u32;
            }
            4 => {
                bits += (bytes[0] as u32) << 24;
                bits += (bytes[1] as u32) << 16;
                bits += (bytes[2] as u32) << 8;
                bits += bytes[3] as u32;
            }
            _ => unreachable!(),
        }

        Ok(Self(bits))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Error)]
pub enum ParseRecordIdError {
    #[error("invalid string length: {0}, expected 4")]
    Length(usize),
    #[error("invalid hex: {0}")]
    InvalidHex(FromHexError),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordReference {
    pub module: ModuleId,
    pub record: RecordId,
}

impl RecordReference {
    pub const STUB: Self = Self {
        module: ModuleId::CORE,
        record: RecordId(0),
    };
}

impl Display for RecordReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.module, self.record)
    }
}

impl FromStr for RecordReference {
    type Err = ParseRecordReferenceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (module, record) = s
            .split_once(':')
            .ok_or(ParseRecordReferenceError::BadFormat)?;

        let module = module
            .parse()
            .map_err(ParseRecordReferenceError::ModuleId)?;
        let record = record
            .parse()
            .map_err(ParseRecordReferenceError::RecordId)?;

        Ok(Self { module, record })
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseRecordReferenceError {
    #[error("bad format, expected one ':'")]
    BadFormat,
    #[error("invalid module id: {0}")]
    ModuleId(ParseModuleIdError),
    #[error("invalid record id: {0}")]
    RecordId(ParseRecordIdError),
}

#[cfg(test)]
mod tests {
    use crate::record::RecordId;

    #[test]
    fn parse_record_id() {
        let input = "ff";
        let output = RecordId(255);

        assert_eq!(input.parse::<RecordId>().unwrap(), output);

        let input = "ff0c";
        let output = RecordId(65292);

        assert_eq!(input.parse::<RecordId>().unwrap(), output);
    }
}
