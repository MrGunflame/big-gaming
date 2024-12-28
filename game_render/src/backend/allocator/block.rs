use core::alloc::Layout;

pub struct BlockAllocator {}

impl BlockAllocator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn alloc(&mut self, layout: Layout) -> usize {
        todo!()
    }

    pub fn dealloc(&mut self) {}
}
