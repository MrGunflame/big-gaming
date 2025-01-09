use std::alloc::{handle_alloc_error, Layout};
use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;

use super::IsZst;

/// A fixed size heap-allocated arena that returns references on insertion.
// This type cannot be Sync insertion takes `&self` and is not thread safe.
// This type maybe can be Send if T: Send + Sync since moving the arena itself
// does not break references, but it is not implemented for now.
// This type cannot implement Clone, since clone takes `&self`, but other references
// may still exist at that time.
#[derive(Debug)]
pub struct ScratchBuffer<T> {
    ptr: NonNull<T>,
    len: Cell<usize>,
    cap: usize,
    // Explicit !Send/!Sync marker. This is already given because of previous fields
    // but is here just in case the implementation changes and the auto-impls change.
    _marker: PhantomData<*const ()>,
}

impl<T> ScratchBuffer<T>
where
    T: Sized,
{
    pub fn new(capacity: usize) -> Self {
        let (ptr, cap) = if T::IS_ZST {
            // Any dangling, but well-aligned pointer is valid
            // for ZSTs.
            (NonNull::dangling(), usize::MAX)
        } else {
            let layout = array_layout::<T>(capacity);
            let ptr = unsafe { std::alloc::alloc(layout) };
            let ptr = NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout));
            (ptr, capacity)
        };

        Self {
            ptr: ptr.cast::<T>(),
            len: Cell::new(0),
            cap,
            _marker: PhantomData,
        }
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn insert(&self, value: T) -> &mut T {
        self.try_insert(value)
            .unwrap_or_else(|_| panic!("ScratchArena is full"))
    }

    pub fn try_insert(&self, value: T) -> Result<&mut T, T> {
        if self.len.get() == self.cap {
            Err(value)
        } else {
            Ok(unsafe { self.insert_unchecked(value) })
        }
    }

    pub unsafe fn insert_unchecked(&self, value: T) -> &mut T {
        debug_assert!(self.len.get() < self.cap);

        unsafe {
            let mut ptr = self.ptr.add(self.len.get());
            self.len.set(self.len.get() + 1);

            ptr.write(value);
            ptr.as_mut()
        }
    }

    /// Returns a slice to all elements in this `ScratchBuffer`.
    ///
    /// Note that this function takes `&mut self` to ensure all references previously returned by
    /// [`insert`] are dropped.
    ///
    /// [`insert`]: Self::insert
    pub fn as_slice(&mut self) -> &[T] {
        let ptr = self.ptr.as_ptr();
        let len = *self.len.get_mut();
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }

    /// Returns a mutable sliec to all elements in this `ScratchBuffer`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let ptr = self.ptr.as_ptr();
        let len = *self.len.get_mut();
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    }

    pub fn truncate(&mut self, new_len: usize) {
        let len = self.len.get();
        if new_len > len {
            return;
        }

        unsafe {
            let s =
                core::ptr::slice_from_raw_parts_mut(self.ptr.add(new_len).as_ptr(), len - new_len);
            // Set the new len before dropping the values.
            // Dropping invokes arbitrary code that may otherwise
            // cause the slice to be half-initialized.
            self.len.set(new_len);
            core::ptr::drop_in_place(s);
        }
    }

    pub fn clear(&mut self) {
        self.truncate(0);
    }
}

impl<T> Drop for ScratchBuffer<T> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() {
            let mut ptr = self.ptr;
            let mut len = self.len.get();
            while len != 0 {
                unsafe {
                    ptr.drop_in_place();
                    ptr = ptr.add(1);
                    len -= 1;
                }
            }
        }

        if !T::IS_ZST {
            let layout = array_layout::<T>(self.cap);
            unsafe {
                std::alloc::dealloc(self.ptr.cast::<u8>().as_ptr(), layout);
            }
        }
    }
}

fn array_layout<T>(len: usize) -> Layout {
    Layout::array::<T>(len).unwrap_or_else(|_| panic!("capacity overflow"))
}

#[cfg(test)]
mod tests {
    use super::ScratchBuffer;

    #[test]
    fn scratch_arena_insert() {
        // Miri test
        let arena = ScratchBuffer::new(16);

        let vals: Vec<_> = (0..16).map(|index| arena.insert(index)).collect();

        for (index, val) in vals.into_iter().enumerate() {
            *val += 1;
            assert_eq!(*val, index + 1);
        }

        drop(arena);
    }

    #[test]
    fn scratch_arena_drop() {
        let arena = ScratchBuffer::new(1);
        let str_ref = arena.insert("Hello World".to_owned());
        assert_eq!(*str_ref, "Hello World");
        drop(arena);
    }

    #[test]
    fn scratch_arena_insert_at_capacity() {
        let arena = ScratchBuffer::new(1);
        arena.try_insert(0).unwrap();
        arena.try_insert(0).unwrap_err();
    }

    #[test]
    fn scratch_arena_zst() {
        let arena = ScratchBuffer::new(1);
        arena.insert(());
        drop(arena);
    }
}
