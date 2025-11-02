use std::alloc::Layout;
use std::ptr::NonNull;

use allocator_api2::alloc::{AllocError, Allocator};
use bumpalo::Bump;

#[derive(Debug, Default)]
pub struct BumpAllocator {
    inner: Bump,
}

impl BumpAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn span(&mut self) -> BumpAllocatorSpan<'_> {
        self.inner.reset();
        BumpAllocatorSpan {
            inner: &mut self.inner,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BumpAllocatorSpan<'a> {
    inner: &'a Bump,
}

unsafe impl<'a> Allocator for BumpAllocatorSpan<'a> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.inner.deallocate(ptr, layout) }
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate_zeroed(layout)
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.inner.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.inner.grow_zeroed(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.inner.shrink(ptr, old_layout, new_layout) }
    }
}
