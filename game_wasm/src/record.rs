use core::mem::MaybeUninit;

use bytemuck::{Pod, Zeroable};

use crate::raw::record::{get_record, RecordKind as RawRecordKind};
use crate::raw::{Ptr, PtrMut};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordReference {
    pub module: ModuleId,
    pub record: RecordId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct ModuleId([u8; 16]);

impl ModuleId {
    pub const CORE: Self = Self([0; 16]);

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct RecordId(pub u32);

#[derive(Clone, Debug)]
pub struct Record {
    kind: RecordKind,
}

impl Record {
    pub fn get(id: RecordReference) -> Self {
        let mut record = MaybeUninit::uninit();

        let res = unsafe { get_record(Ptr::from_ptr(&id), PtrMut::from_ptr(record.as_mut_ptr())) };
        assert!(res == 0);

        let record = unsafe { record.assume_init() };
        Self {
            kind: RecordKind::from_raw(record.kind),
        }
    }

    #[inline]
    pub const fn kind(&self) -> RecordKind {
        self.kind
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
        }
    }
}
