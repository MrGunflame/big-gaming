use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;

use allocator_api2::alloc::Allocator;
use allocator_api2::boxed::Box;

/// A queue with a fixed size.
// The scheduler has a queue of nodes that are ready. In each iteration the
// scheduler will do the following operations:
// 1. Drain the queue
// 2. Push nodes to the queue that have become ready in this iteration.
// The 2. step needs to push new nodes to the queue, however the nodes
// in the current iteration still need to be kept.
// `Queue` takes advantage of the fact that we know the number of nodes beforehand
// and can create a slice with fixed size that allows pushing new nodes
// while other nodes are still borrowed from the same slice.
#[derive(Debug)]
pub struct Queue<T, A>
where
    // If T is Copy we can skip complicated drop logic.
    T: Copy,
    A: Allocator,
{
    inner: Box<[UnsafeCell<MaybeUninit<T>>], A>,
    offset: Cell<usize>,
    len: Cell<usize>,
}

impl<T, A> Queue<T, A>
where
    T: Copy,
    A: Allocator,
{
    /// Creates a new `Queue` with a given size.
    pub fn new_in(len: usize, alloc: A) -> Self {
        // We cannot directly cast from `Box<[MaybeUninit<T>]>` to
        // `Box<[UnsafeCell<MaybeUninit<T>>]>` since the `Box` has no layout guarantees
        // if we include a allocator.
        // Instead we must destructure the Box, cast the pointer and then reconstruct
        // the Box from the new pointer and allocator.
        let boxed = Box::<[T], A>::new_uninit_slice_in(len, alloc);
        let (ptr, alloc) = Box::into_raw_with_allocator(boxed);

        // UnsafeCell is `#[repr(transparent)]`, so we can cast from `T` to `UnsafeCell<T>`.
        let ptr = ptr as *mut [UnsafeCell<MaybeUninit<T>>];

        let inner = unsafe { Box::from_raw_in(ptr, alloc) };

        Self {
            inner,
            offset: Cell::new(0),
            len: Cell::new(0),
        }
    }

    /// Pushes a new lement to the back of the `Queue`.
    pub fn push(&self, elem: T) {
        let offset = self.offset.get();
        let len = self.len.get();
        assert!(offset + len < self.inner.len());

        unsafe {
            self.push_unchecked(elem);
        }
    }

    /// Pushes a new element to the back of the `Queue`.
    ///
    /// # Safety
    ///
    /// The `Queue` has a fixed size on creation, which is the maximum number of elements that can
    /// be pushed. After the size is exhausted this function must not be called again.
    pub unsafe fn push_unchecked(&self, elem: T) {
        let offset = self.offset.get();
        let len = self.len.get();

        unsafe {
            let index = offset.unchecked_add(len);
            (*self.inner.get_unchecked(index).get()).write(elem);

            self.len.set(len.unchecked_add(1));
        }
    }

    /// Drains and returns the current elements in the `Queue`.
    pub fn take_and_advance(&self) -> &[T] {
        let offset = self.offset.get();
        let len = self.len.get();

        self.offset.set(offset + len);
        self.len.set(0);

        unsafe {
            // Get a pointer to the start of the queue section.
            // The range offset..offset+len is initialized and
            // as such we can cast the pointer to the init value.
            let ptr = self.inner.as_ptr().add(offset);
            let ptr = ptr.cast::<T>();

            core::slice::from_raw_parts(ptr, len)
        }
    }
}

#[cfg(test)]
mod tests {
    use allocator_api2::alloc::Global;

    use super::Queue;

    #[test]
    fn queue_push_take() {
        let queue = Queue::new_in(4, Global);

        queue.push(0);
        queue.push(1);
        let sl0 = queue.take_and_advance();
        queue.push(2);
        queue.push(3);
        let sl1 = queue.take_and_advance();

        assert_eq!(sl0, [0, 1]);
        assert_eq!(sl1, [2, 3]);
    }
}
