use std::time::Duration;

use hashbrown::HashMap;
use parking_lot::RwLock;
use slab::Slab;

#[derive(Debug, Default)]
pub struct Statistics {
    pub gpu_timings: RwLock<Timings>,
    pub memory: RwLock<MemoryStatistics>,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryStatistics {
    pub blocks: Slab<MemoryBlock>,
}

#[derive(Clone, Debug)]
pub struct MemoryBlock {
    pub size: u64,
    pub used: u64,
    pub allocs: HashMap<u64, MemoryAlloc>,
    pub dedicated: bool,
    pub device_local: bool,
    pub host_visible: bool,
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

#[derive(Clone, Debug, Default)]
pub struct Timings {
    pub time: Duration,
    pub passes: Vec<Pass>,
}

#[derive(Clone, Debug)]
pub struct Pass {
    pub name: &'static str,
    pub time: Duration,
}
