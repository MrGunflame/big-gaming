//! Binary (de)serialization format

use std::mem;
use std::ptr;

pub use game_macros::Encode;

/// A type that can be encoded into a binary buffer.
///
/// **Note that the pointer may be unaligned.**
pub unsafe trait Encode {
    /// Returns the exact size required to encode the type.
    fn size(&self) -> usize;

    /// Encodes the value into the given buffer.
    ///
    /// # Safety
    ///
    /// This pointer to the buffer must be valid for the number of bytes returned by [`size`]..
    ///
    /// [`size`]: Self::size
    unsafe fn encode(&self, buf: *mut u8);
}

macro_rules! impl_encode_int {
    ($($t:ty),*) => {
        $(
            unsafe impl Encode for $t {
                #[inline]
                fn size(&self) -> usize {
                    mem::size_of::<Self>()
                }

                #[inline]
                unsafe fn encode(&self, buf: *mut u8) {
                    let bytes = self.to_le_bytes();

                    unsafe {
                        ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
                    }
                }
            }
        )*
    };
}

impl_encode_int! { u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize }

unsafe impl Encode for [u8] {
    fn size(&self) -> usize {
        self.len().size() + self.len()
    }

    unsafe fn encode(&self, buf: *mut u8) {
        unsafe {
            ptr::copy_nonoverlapping(self.as_ptr(), buf, self.len());
        }
    }
}

unsafe impl Encode for str {
    fn size(&self) -> usize {
        self.len()
    }

    unsafe fn encode(&self, buf: *mut u8) {
        unsafe {
            self.as_bytes().encode(buf);
        }
    }
}
