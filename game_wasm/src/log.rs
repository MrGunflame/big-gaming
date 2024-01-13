use crate::raw;

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
    unsafe {
        raw::log(level.0, content.as_ptr(), content.len());
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        extern crate alloc;
        let level = $crate::log::Level::ERROR;
        let content = alloc::format!($($arg)*);
        $crate::log::log(level, &content);
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        extern crate alloc;
        let level = $crate::log::Level::WARN;
        let content = alloc::format!($($arg)*);
        $crate::log::log(level, &content);
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        extern crate alloc;
        let level = $crate::log::Level::INFO;
        let content = alloc::format!($($arg)*);
        $crate::log::log(level, &content);
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        extern crate alloc;
        let level = $crate::log::Level::DEBUG;
        let content = alloc::format!($($arg)*);
        $crate::log::log(level, &content);
    }};
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        extern crate alloc;
        let level = $crate::log::Level::TRACE;
        let content = alloc::format!($($arg)*);
        $crate::log::log(level, &content);
    }};
}
