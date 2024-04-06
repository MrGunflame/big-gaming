use core::fmt::{self, Display, Formatter, LowerHex};
use core::mem::MaybeUninit;
use core::str::FromStr;

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
use hex::FromHexError;

use crate::components::{Components, RawComponent};
use crate::encoding::{Decode, Encode};
use crate::raw::record::{
    get_record, get_record_component_get, get_record_component_keys, get_record_component_len,
    get_record_len_component, RecordKind as RawRecordKind,
};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod, Encode, Decode)]
#[repr(transparent)]
pub struct RecordId(pub u32);

#[derive(Clone, Debug)]
pub struct Record {
    pub(crate) kind: RecordKind,
    pub(crate) components: Components,
}

impl Record {
    pub fn get(id: RecordReference) -> Self {
        let mut record = MaybeUninit::uninit();

        let res = unsafe { get_record(&id, record.as_mut_ptr()) };
        assert!(res == 0);

        let record = unsafe { record.assume_init() };
        Self {
            kind: RecordKind::from_raw(record.kind),
            components: fetch_components(id),
        }
    }

    #[inline]
    pub const fn kind(&self) -> RecordKind {
        self.kind
    }

    #[inline]
    pub const fn components(&self) -> &Components {
        &self.components
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    Object,
    Item,
    Race,
}

impl RecordKind {
    fn from_raw(kind: RawRecordKind) -> Self {
        match kind {
            RawRecordKind::ITEM => Self::Item,
            RawRecordKind::OBJECT => Self::Object,
            RawRecordKind::RACE => Self::Race,
            _ => unreachable!(),
        }
    }
}

fn fetch_components(id: RecordReference) -> Components {
    let mut len = MaybeUninit::uninit();

    let res = unsafe { get_record_len_component(&id, len.as_mut_ptr()) };
    assert!(res == 0);

    let len = unsafe { len.assume_init() };

    let mut keys = Vec::with_capacity(len as usize);
    let res = unsafe { get_record_component_keys(&id, keys.as_mut_ptr(), len) };
    assert!(res == 0);
    unsafe { keys.set_len(len as usize) };

    // let mut components = Components::new();
    // for key in keys.into_iter() {
    //     let comp = fetch_component(id, key);
    //     components.insert(key, comp);
    // }
    todo!();

    // components
}

fn fetch_component(id: RecordReference, component: RecordReference) -> RawComponent {
    let mut len = MaybeUninit::uninit();

    let res = unsafe { get_record_component_len(&id, &component, len.as_mut_ptr()) };
    assert!(res == 0);

    let len = unsafe { len.assume_init() };
    let mut bytes = Vec::with_capacity(len as usize);

    let res = unsafe { get_record_component_get(&id, &component, bytes.as_mut_ptr(), len) };
    assert!(res == 0);

    unsafe { bytes.set_len(len as usize) };

    RawComponent::new(bytes, Vec::new())
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
