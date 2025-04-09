use std::collections::HashMap;

use parking_lot::RwLock;
use slab::Slab;

#[derive(Debug, Default)]
pub struct Statistics {
    pub memory: RwLock<MemoryStatistics>,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryStatistics {
    pub blocks: Slab<MemoryBlock>,
}

#[derive(Clone, Debug)]
pub struct MemoryBlock {
    pub size: u64,
    pub allocs: HashMap<u64, MemoryAlloc>,
}

#[derive(Clone, Debug)]
pub struct MemoryAlloc {
    pub offset: u64,
    pub size: u64,
    pub kind: AllocationKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllocationKind {
    Buffer,
    Texture,
}
