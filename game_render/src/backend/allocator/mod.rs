mod block;
mod buddy;

use std::alloc::Layout;

pub use block::BlockAllocator;
pub use buddy::BuddyAllocator;

use super::vulkan::Device;
use super::MemoryRequirements;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllocationError {
    OutOfMemory,
}

pub trait Allocator {
    fn alloc(&mut self, layout: Layout) -> Option<Region>;

    unsafe fn dealloc(&mut self, region: Region);
}

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

pub struct GeneralPurposeAllocator {
    device: Device,
}

impl GeneralPurposeAllocator {
    pub fn new(device: Device) -> Self {
        Self { device }
    }
}

pub struct AllocatedBuffer {}
