use std::alloc::{GlobalAlloc, Layout};

use tracy_client::Client;

pub struct ProfiledAllocator<T>(T);

impl<T> ProfiledAllocator<T> {
    pub const fn new(inner_allocator: T) -> Self {
        Self(inner_allocator)
    }

    fn emit_alloc(&self, ptr: *mut u8, size: usize) {
        if !Client::is_running() {
            return;
        }

        unsafe {
            tracy_client::sys::___tracy_emit_memory_alloc(ptr.cast(), size, 1);
        }
    }

    fn emit_free(&self, ptr: *mut u8) {
        if !Client::is_running() {
            return;
        }

        unsafe {
            tracy_client::sys::___tracy_emit_memory_free(ptr.cast(), 1);
        }
    }
}

unsafe impl<T> GlobalAlloc for ProfiledAllocator<T>
where
    T: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.0.alloc(layout) };
        self.emit_alloc(ptr, layout.size());
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.emit_free(ptr);
        unsafe {
            self.0.dealloc(ptr, layout);
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.0.alloc_zeroed(layout) };
        self.emit_alloc(ptr, layout.size());
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.emit_free(ptr);
        let ptr = unsafe { self.0.realloc(ptr, layout, new_size) };
        self.emit_alloc(ptr, new_size);
        ptr
    }
}
