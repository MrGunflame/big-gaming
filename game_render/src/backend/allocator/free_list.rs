use std::alloc::Layout;

use super::{Allocator, GrowableAllocator, Region};

#[derive(Clone, Debug)]
pub struct FreeListAllocator {
    chunks: Vec<Chunk>,
}

impl Allocator for FreeListAllocator {
    fn new(region: Region) -> Self {
        Self {
            chunks: vec![Chunk {
                offset: region.offset,
                size: region.offset + region.size,
                is_free: true,
            }],
        }
    }

    fn alloc(&mut self, layout: Layout) -> Option<Region> {
        for chunk in &mut self.chunks {
            todo!()
        }

        None
    }

    unsafe fn dealloc(&mut self, region: Region) {}
}

#[derive(Copy, Clone, Debug)]
struct Chunk {
    offset: usize,
    size: usize,
    is_free: bool,
}
