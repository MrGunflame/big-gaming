mod block;
mod buddy;

pub use block::BlockAllocator;
pub use buddy::BuddyAllocator;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllocationError {
    OutOfMemory,
}

pub trait Allocator {}

#[derive(Copy, Clone, Debug)]
pub struct Region {
    offset: usize,
    size: usize,
}

impl Region {
    pub const fn new(offset: usize, size: usize) -> Self {
        Self { offset, size }
    }

    pub const fn start(&self) -> usize {
        self.offset
    }

    pub const fn end(&self) -> usize {
        self.offset + self.size
    }
}
