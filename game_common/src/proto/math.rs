//! Impls for math types
//!
use std::{mem, ptr};

use glam::{Quat, Vec2, Vec3};

use super::Encode;

unsafe impl Encode for Vec2 {
    fn size(&self) -> usize {
        mem::size_of::<f32>() * 2
    }

    unsafe fn encode(&self, buf: *mut u8) {
        let array = self.to_array();

        unsafe {
            ptr::copy_nonoverlapping(array.as_ptr().cast(), buf, 8);
        }
    }
}

unsafe impl Encode for Vec3 {
    fn size(&self) -> usize {
        mem::size_of::<f32>() * 3
    }

    unsafe fn encode(&self, buf: *mut u8) {
        let array = self.to_array();

        unsafe {
            ptr::copy_nonoverlapping(array.as_ptr().cast(), buf, 12);
        }
    }
}

unsafe impl Encode for Quat {
    fn size(&self) -> usize {
        mem::size_of::<f32>() * 4
    }

    unsafe fn encode(&self, buf: *mut u8) {
        let array = self.to_array();

        unsafe {
            ptr::copy_nonoverlapping(array.as_ptr().cast(), buf, 16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Encode, Vec2, Vec3};

    #[test]
    fn encode_vec2() {
        let value = Vec2::new(0.0, 0.0);

        let mut buf = vec![0u8; value.size()];

        unsafe {
            value.encode(buf.as_mut_ptr());
        }

        assert_eq!(buf, [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_vec3() {
        let value = Vec3::new(0.0, 0.0, 0.0);

        let mut buf = vec![0u8; value.size()];

        unsafe {
            value.encode(buf.as_mut_ptr());
        }

        assert_eq!(buf, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }
}
