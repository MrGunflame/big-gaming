mod block;
mod buddy;

use std::alloc::Layout;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Arc;

pub use block::BlockAllocator;
pub use buddy::BuddyAllocator;
use parking_lot::Mutex;
use slab::Slab;

use super::vulkan::{Buffer, Device, DeviceMemory, DeviceMemorySlice, Texture};
use super::{BufferUsage, MemoryRequirements, TextureDescriptor};

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

const MIN_SIZE: NonZeroU64 = NonZeroU64::new(8192).unwrap();
const MAX_SIZE: NonZeroU64 = NonZeroU64::new(u32::MAX as u64).unwrap();
const GROWTH_FACTOR: NonZeroU64 = NonZeroU64::new(2).unwrap();

#[derive(Clone, Debug)]
pub struct GeneralPurposeAllocator {
    device: Device,
    inner: Arc<Mutex<GpAllocatorInner>>,
}

impl GeneralPurposeAllocator {
    pub fn new(device: Device) -> Self {
        Self {
            device,
            inner: Arc::new(Mutex::new(GpAllocatorInner {
                pools: HashMap::new(),
            })),
        }
    }

    pub fn create_buffer(&self, size: NonZeroU64, usage: BufferUsage) -> Buffer {
        let mut buffer = self.device.create_buffer(size, usage);
        let req = self.device.buffer_memory_requirements(&buffer);
        let memory = self.alloc(req);
        unsafe {
            self.device.bind_buffer_memory(&mut buffer, memory.memory());
        }

        buffer
    }

    pub fn create_texture(&self, descriptor: &TextureDescriptor) -> Texture {
        let mut texture = self.device.create_texture(descriptor);
        let req = self.device.image_memory_requirements(&texture);
        let memory = self.alloc(req);
        unsafe {
            self.device
                .bind_texture_memory(&mut texture, memory.memory());
        }

        texture
    }

    pub fn alloc(&self, req: MemoryRequirements) -> DeviceMemoryRegion {
        let inner = &mut *self.inner.lock();

        for mem_typ in req.memory_types {
            // Note: entry API does not work since we need to borrow inner
            // multiple times, so we use contains_key and get instead.
            if !inner.pools.contains_key(&mem_typ) {
                let memory = self.device.allocate_memory(MIN_SIZE, mem_typ);
                let mut blocks = Slab::new();
                let last = blocks.insert(Block {
                    memory: Arc::new(memory),
                    allocator: BuddyAllocator::new(Region {
                        offset: 0,
                        size: MIN_SIZE.get() as usize,
                    }),
                    num_allocs: 0,
                    size: MIN_SIZE,
                });

                inner.pools.insert(mem_typ, Pool { blocks, last });
            }

            let pool = inner.pools.get_mut(&mem_typ).unwrap();

            for (block_index, block) in &mut pool.blocks {
                let Some(region) = block.alloc(req.size, req.align) else {
                    continue;
                };

                return DeviceMemoryRegion {
                    allocator: self.clone(),
                    memory: block.memory.clone(),
                    region,
                    memory_type: mem_typ,
                    block_index,
                };
            }

            let prev_size = pool.blocks[pool.last].size;
            let new_size = core::cmp::max(
                prev_size.saturating_mul(GROWTH_FACTOR),
                req.size.checked_next_power_of_two().unwrap(),
            );
            let memory = self.device.allocate_memory(new_size, mem_typ);

            let block = Block {
                allocator: BuddyAllocator::new(Region {
                    offset: 0,
                    size: new_size.get() as usize,
                }),
                memory: Arc::new(memory),
                num_allocs: 0,
                size: new_size,
            };
            pool.last = pool.blocks.insert(block);

            let block = &mut pool.blocks[pool.last];
            let Some(region) = block.alloc(req.size, req.align) else {
                continue;
            };

            return DeviceMemoryRegion {
                allocator: self.clone(),
                memory: block.memory.clone(),
                region,
                memory_type: mem_typ,
                block_index: pool.last,
            };
        }

        todo!()
    }

    unsafe fn dealloc(&self, mem_typ: u32, block_index: usize, region: Region) {
        let mut inner = self.inner.lock();

        let pool = inner.pools.get_mut(&mem_typ).unwrap();
        let block = pool.blocks.get_mut(block_index).unwrap();

        unsafe {
            block.allocator.dealloc(region);
        }

        block.num_allocs -= 1;
        if block.num_allocs != 0 {
            return;
        }

        pool.blocks.remove(block_index);
        if block_index == pool.last {
            pool.last = pool.blocks.iter().map(|(index, _)| index).last().unwrap();
        }
    }
}

#[derive(Debug)]
struct GpAllocatorInner {
    pools: HashMap<u32, Pool>,
}

#[derive(Debug)]
struct Pool {
    blocks: Slab<Block>,
    last: usize,
}

#[derive(Debug)]
struct Block {
    memory: Arc<DeviceMemory>,
    allocator: BuddyAllocator,
    num_allocs: usize,
    size: NonZeroU64,
}

impl Block {
    fn alloc(&mut self, size: NonZeroU64, align: NonZeroU64) -> Option<Region> {
        let region = self
            .allocator
            .alloc(Layout::from_size_align(size.get() as usize, align.get() as usize).unwrap())?;
        self.num_allocs += 1;
        Some(region)
    }
}

pub struct DeviceMemoryRegion {
    allocator: GeneralPurposeAllocator,
    memory: Arc<DeviceMemory>,
    region: Region,
    memory_type: u32,
    block_index: usize,
}

impl DeviceMemoryRegion {
    pub fn memory(&self) -> DeviceMemorySlice<'_> {
        self.memory
            .slice(self.region.offset as u64..self.region.offset as u64 + self.region.size as u64)
    }
}

impl Drop for DeviceMemoryRegion {
    fn drop(&mut self) {
        unsafe {
            self.allocator
                .dealloc(self.memory_type, self.block_index, self.region);
        }
    }
}
