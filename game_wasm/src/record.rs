use core::fmt::{self, Display, Formatter, LowerHex};
use core::str::FromStr;

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
use hex::FromHexError;

use crate::encoding::{Decode, Encode};
use crate::raw::record::{record_data_copy, record_data_len};
use crate::raw::{RESULT_NO_RECORD, RESULT_OK};
use crate::{unreachable_unchecked, Error, ErrorImpl};

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod, Encode, Decode)]
#[repr(C)]
pub struct RecordReference {
    pub module: ModuleId,
    pub record: RecordId,
}

impl RecordReference {
    pub const STUB: RecordReference = RecordReference {
        module: ModuleId::CORE,
        record: RecordId(0),
    };

    pub const fn from_str_const(s: &str) -> Self {
        let mut index = 0;
        while index < s.len() {
            if s.as_bytes()[index] == b':' {
                break;
            }

            index += 1;
        }

        let module = ModuleId::from_str_const_with_offset(s, 0, index);
        let record = RecordId::from_str_const_with_offset(s, index + 1, s.len());
        Self { module, record }
    }
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod, Encode, Decode)]
#[repr(transparent)]
pub struct RecordId(pub u32);

impl RecordId {
    pub const fn from_str_const(s: &str) -> Self {
        Self::from_str_const_with_offset(s, 0, s.len())
    }

    const fn from_str_const_with_offset(s: &str, start: usize, end: usize) -> Self {
        let mut value = 0;

        let mut index = 0;
        while start + index < end {
            let b = s.as_bytes()[start + index];

            let nibble = match b {
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

            value = (value << 4) + nibble;
            index += 1;
        }

        Self(value)
    }
}

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl FromStr for RecordId {
    type Err = ParseRecordIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(ParseRecordIdError::InvalidHex)?;

        if bytes.is_empty() || bytes.len() > 4 {
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ParseRecordIdError {
    Length(usize),
    InvalidHex(FromHexError),
}

impl Display for ParseRecordIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length(len) => write!(f, "invalid string length: {}, expected 4", len),
            Self::InvalidHex(err) => write!(f, "invalid hex: {}", err),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ParseRecordReferenceError {
    BadFormat,
    ModuleId(ParseModuleIdError),
    RecordId(ParseRecordIdError),
}

impl Display for ParseRecordReferenceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadFormat => write!(f, "bad format, expected one ':'"),
            Self::ModuleId(err) => write!(f, "invalid module id: {}", err),
            Self::RecordId(err) => write!(f, "invalid record id: {}", err),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod, Encode, Decode)]
#[repr(transparent)]
pub struct ModuleId([u8; 16]);

impl ModuleId {
    pub const CORE: Self = Self([0; 16]);

    #[inline]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    #[inline]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn from_str_const(s: &str) -> Self {
        Self::from_str_const_with_offset(s, 0, s.len())
    }

    /// start inclusive, end exclusive
    const fn from_str_const_with_offset(s: &str, start: usize, end: usize) -> Self {
        let mut bytes = [0; 16];

        let buf = s.as_bytes();
        if end - start != 32 {
            panic!("invalid string length");
        }

        let mut index = 0;
        while index < 32 {
            let b = buf[start + index];

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParseModuleIdError {
    InvalidLength(usize),
    InvalidByte { byte: u8, position: usize },
}

impl Display for ParseModuleIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(len) => write!(f, "invalid length: {}", len),
            Self::InvalidByte { byte, position } => {
                write!(f, "invalid byte {} at position {}", byte, position)
            }
        }
    }
}

impl FromStr for ModuleId {
    type Err = ParseModuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            return Err(ParseModuleIdError::InvalidLength(s.len()));
        }

        let mut bytes = [0; 16];

        for (index, &byte) in s.as_bytes().iter().enumerate() {
            let mut nibble = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => 10 + byte - b'a',
                b'A'..=b'F' => 10 + byte - b'A',
                _ => {
                    return Err(ParseModuleIdError::InvalidByte {
                        byte,
                        position: index,
                    })
                }
            };

            if index % 2 == 0 {
                nibble <<= 4;
            }

            bytes[index / 2] += nibble;
        }

        Ok(Self::from_bytes(bytes))
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.into_bytes() {
            let high = HEX_CHARS[((byte & 0xf0) >> 4) as usize];
            let low = HEX_CHARS[(byte & 0x0f) as usize];
            write!(f, "{}{}", high as char, low as char)?;
        }

        Ok(())
    }
}

/// A record, exported by a module.
#[derive(Clone, Debug)]
pub struct Record {
    data: Vec<u8>,
}

impl Record {
    /// Returns the `Record` with the given `id`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if no `Record` with the given `id` exists.
    pub fn get(id: RecordReference) -> Result<Self, Error> {
        let data = get_record_data_safe(id)?;
        Ok(Self { data })
    }

    /// Returns a reference to the data of the `Record`.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

fn get_record_data_safe(id: RecordReference) -> Result<Vec<u8>, Error> {
    let mut len = 0;

    match unsafe { record_data_len(&id, &mut len) } {
        RESULT_OK => (),
        RESULT_NO_RECORD => return Err(Error(ErrorImpl::NoRecord(id))),
        _ => unreachable!(),
    }

    let mut data = Vec::with_capacity(len);

    match unsafe { record_data_copy(&id, data.as_mut_ptr(), len) } {
        // This operation will always suceed as the only failure
        // condition is that the record does not exist. We have
        // already checked for that previously. Since records are
        // immutable checking whether the record exists only means
        // that it will always exist.
        RESULT_OK => (),
        _ => unsafe { unreachable_unchecked() },
    }

    unsafe {
        data.set_len(len);
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::{ModuleId, RecordId};

    #[test]
    fn module_id_parse_from_display() {
        let id = ModuleId::from_bytes([
            0xc6, 0x26, 0xb9, 0xb0, 0xab, 0x19, 0x40, 0xab, 0xa6, 0x93, 0x2e, 0xa7, 0x72, 0x6d,
            0x01, 0x75,
        ]);

        let string = id.to_string();
        assert_eq!(string.parse::<ModuleId>(), Ok(id));
    }

    #[test]
    fn parse_record_id() {
        let input = "ff";
        let output = RecordId(255);

        assert_eq!(input.parse::<RecordId>().unwrap(), output);

        let input = "ff0c";
        let output = RecordId(65292);

        assert_eq!(input.parse::<RecordId>().unwrap(), output);
    }

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
