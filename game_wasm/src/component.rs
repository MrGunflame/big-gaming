use core::mem;
use core::ptr::NonNull;

use alloc::vec::Vec;
use bytemuck::{AnyBitPattern, NoUninit, Pod};

/// A byte buffer containing component data.
///
/// Note that the buffer has the alignment of `u8`. If you read values from the buffer you must use
/// [`read_unaligned`].
///
/// [`read_unaligned`]: ptr::read_unaligned
#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    bytes: Vec<u8>,
}

impl Component {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Reads the value `T` from the buffer.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not big enough to hold `T`.
    #[inline]
    pub fn read<T>(&self) -> T
    where
        T: AnyBitPattern,
    {
        assert!(self.len() >= mem::size_of::<T>());

        // SAFETY: We have validated that the buffer is big enough for `T`.
        unsafe { self.read_unchecked() }
    }

    /// Reads the value `T` from the buffer without checking that the buffer is big enough.
    ///
    /// Note that the read is always unaligned and the buffer must not be correctly aligned for `T`.
    ///
    /// # Safety
    ///
    /// The buffer must have at least `mem::size_of::<T>` bytes.
    #[inline]
    pub unsafe fn read_unchecked<T>(&self) -> T
    where
        T: AnyBitPattern,
    {
        debug_assert!(self.bytes.len() >= mem::size_of::<T>());

        // SAFETY: `T` implements `AnyBitPattern`, which means any
        // read possible value is inhabitet.
        // The caller guarantees that `bytes.len() >= size_of::<T>()`.
        unsafe { (self.bytes.as_ptr() as *const T).read_unaligned() }
    }

    pub fn write<T>(&mut self, value: T)
    where
        T: NoUninit,
    {
        let arr = &[value];
        let slice: &[u8] = bytemuck::cast_slice(arr);

        self.bytes.resize(slice.len(), 0);

        assert!(self.bytes.len() >= slice.len());

        unsafe {
            let dst = self.bytes.as_mut_ptr();
            let src = slice.as_ptr();
            let count = slice.len();

            core::ptr::copy_nonoverlapping(src, dst, count);
        }
    }

    pub unsafe fn write_unchecked<T>(&mut self, value: T)
    where
        T: NoUninit,
    {
        if T::IS_ZST {
            return;
        }

        let slice = bytemuck::bytes_of(&value);

        unsafe {
            let dst = self.bytes.as_mut_ptr();
            let src = slice.as_ptr();
            let count = slice.len();

            core::ptr::copy_nonoverlapping(src, dst, count);
        }
    }

    pub fn update<T, U, F>(&mut self, f: F) -> U
    where
        T: Pod,
        F: FnOnce(&mut T) -> U,
    {
        if T::IS_ZST {
            // Any correctly aligned non-zero pointer is valid for ZST `T`s.
            let mut ptr = NonNull::<T>::dangling();
            let val = unsafe { ptr.as_mut() };
            return f(val);
        }

        assert!(self.bytes.len() >= mem::size_of::<T>());

        // If the buffer is already correctly aligned for `T` we can just
        // cast the pointer into `self.bytes` to `T`.
        // Otherwise we need to copy and write back the value.

        // Also note that some `T`s are always aligned.

        let ptr = self.bytes.as_mut_ptr();

        if ptr.align_offset(mem::align_of::<T>()) == 0 {
            let value = unsafe { &mut *(ptr as *mut T) };
            f(value)
        } else {
            let mut value = unsafe { self.read_unchecked() };
            let res = f(&mut value);
            unsafe { self.write_unchecked(value) };
            res
        }
    }
}

impl AsRef<[u8]> for Component {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

trait IsZst {
    const IS_ZST: bool;
}

impl<T> IsZst for T {
    const IS_ZST: bool = mem::size_of::<Self>() == 0;
}

#[cfg(test)]
mod tests {
    use core::mem;

    use alloc::vec;
    use alloc::vec::Vec;
    use bytemuck::{Pod, Zeroable};

    use super::Component;

    #[test]
    fn component_update_zst() {
        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(transparent)]
        struct Target;

        let mut component = Component { bytes: Vec::new() };
        component.update::<Target, _, _>(|val| {
            *val = Target;
        });

        assert_eq!(component.bytes, vec![]);
    }

    #[test]
    fn component_update_aligned() {
        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(C, align(1))]
        struct Target(u8);

        let mut component = Component { bytes: vec![0] };
        assert!(
            component
                .bytes
                .as_ptr()
                .align_offset(mem::align_of::<Target>())
                == 0
        );

        component.update::<Target, _, _>(|val| {
            *val = Target(1);
        });

        assert_eq!(component.bytes, vec![1]);
    }

    #[test]
    fn component_update_not_aligned() {
        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(C, align(8))]
        struct Target([u8; 32]);

        // If the buffer is aligned, manually "unalign" it by moving the pointer 1 byte
        // forward.
        let mut buf = vec![0; 64];
        let is_aligned = buf.as_ptr().align_offset(mem::align_of::<Target>()) == 0;
        if is_aligned {
            // TODO: Can use `Vec::into_raw_parts` once stable.
            let ptr = buf.as_mut_ptr();
            let len = buf.len();
            let cap = buf.capacity();

            mem::forget(buf);

            buf = unsafe { Vec::from_raw_parts(ptr.add(1), len - 1, cap - 1) };
        }

        let mut component = Component { bytes: buf };
        assert!(
            component
                .bytes
                .as_ptr()
                .align_offset(mem::align_of::<Target>())
                != 0
        );

        component.update::<Target, _, _>(|val| {
            *val = Target([1; 32]);
        });

        // If the buffer was orignally aligned we have to truncate the first
        // byte.
        let mut output = if is_aligned { vec![0; 63] } else { vec![0; 64] };
        for index in 0..32 {
            output[index] = 1;
        }

        assert_eq!(component.bytes, output);

        // Drop the orignal buffer so miri shuts up about leaks.
        if is_aligned {
            let ptr = component.bytes.as_mut_ptr();
            let len = component.bytes.len();
            let cap = component.bytes.capacity();

            mem::forget(component);

            drop(unsafe { Vec::from_raw_parts(ptr.sub(1), len + 1, cap + 1) });
        };
    }
}
