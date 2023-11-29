use core::ops::Deref;

use std::mem::MaybeUninit;
use std::sync::Arc;

const INLINE_CAP: usize = 22;

/// Stack-allocated for small strings.
#[derive(Clone, Debug)]
pub struct SmallStr {
    buffer: Buffer,
}

impl SmallStr {
    #[inline]
    pub const fn from_static(string: &'static str) -> Self {
        Self {
            buffer: Buffer::Static(string),
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.buffer {
            Buffer::Static(s) => s,
            Buffer::Inline(arr, len) => unsafe {
                core::str::from_utf8_unchecked(arr.get_unchecked(..*len as usize))
            },
            Buffer::Shared(s) => &s,
        }
    }
}

impl From<String> for SmallStr {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&str> for SmallStr {
    fn from(src: &str) -> Self {
        if src.len() <= INLINE_CAP {
            let mut arr: [u8; INLINE_CAP] = unsafe { MaybeUninit::zeroed().assume_init() };
            unsafe { core::ptr::copy_nonoverlapping(src.as_ptr(), arr.as_mut_ptr(), src.len()) };

            Self {
                buffer: Buffer::Inline(arr, src.len() as u8),
            }
        } else {
            Self {
                buffer: Buffer::Shared(Arc::from(src)),
            }
        }
    }
}

impl Deref for SmallStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[derive(Debug)]
enum Buffer {
    Static(&'static str),
    Inline([u8; INLINE_CAP], u8),
    Shared(Arc<str>),
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        match self {
            Self::Static(s) => Buffer::Static(s),
            Self::Inline(arr, len) => Buffer::Inline(*arr, *len),
            Self::Shared(s) => Buffer::Shared(s.clone()),
        }
    }
}

impl PartialEq for SmallStr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for SmallStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SmallStr {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Eq for SmallStr {}

#[cfg(test)]
mod tests {
    use super::SmallStr;

    #[test]
    fn smallstr_from_string_inline() {
        let msg = "Hello World!";
        let string = String::from(msg);
        let smallstr = SmallStr::from(string);
        assert_eq!(smallstr, msg);
    }

    #[test]
    fn smallstr_from_string_heap() {
        let msg = "Hello World!Hello World!Hello World!Hello World!Hello World!Hello World!";
        let string = String::from(msg);
        let smallstr = SmallStr::from(string);
        assert_eq!(smallstr, msg);
    }
}
