use std::alloc::Layout;

use super::{Allocator, Region};

pub struct BumpAllocator {
    region: Region,
    next: usize,
}

impl BumpAllocator {
    pub fn reset(&mut self) {
        self.next = self.region.offset;
    }
}

impl Allocator for BumpAllocator {
    fn new(region: Region) -> Self {
        Self {
            region,
            next: region.offset,
        }
    }

    fn alloc(&mut self, layout: Layout) -> Option<Region> {
        let offset = layout.align() - (self.next % layout.align());

        let start = self.next + offset;
        let end = start + layout.size();
        if end > self.region.end() {
            return None;
        }

        self.next = end;
        Some(Region {
            offset: start,
            size: layout.size(),
        })
    }

    unsafe fn dealloc(&mut self, _region: Region) {}
}
