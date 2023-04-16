use crate::raw::{self, Usize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Level(u32);

impl Level {
    pub fn from_raw(level: u32) -> Self {
        Self(level)
    }
}

impl Level {
    pub const ERROR: Self = Self(1);
    pub const WARN: Self = Self(2);
    pub const INFO: Self = Self(3);
    pub const DEBUG: Self = Self(4);
    pub const TRACE: Self = Self(5);
}

pub fn log(level: Level, content: &str) {
    let ptr = content.as_ptr() as Usize;
    let len = content.len() as Usize;

    unsafe {
        raw::log(level.0, ptr, len);
    }
}
