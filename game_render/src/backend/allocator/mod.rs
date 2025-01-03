mod block;
mod buddy;

use std::alloc::Layout;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::Arc;

use bitflags::bitflags;
pub use block::BlockAllocator;
pub use buddy::BuddyAllocator;
use futures_lite::stream::Map;
use parking_lot::Mutex;
use slab::Slab;

use crate::backend::MemoryTypeFlags;

use super::vulkan::{Buffer, Device, DeviceMemory, DeviceMemorySlice, Texture};
use super::{AdapterMemoryProperties, BufferUsage, MemoryRequirements, TextureDescriptor};

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
    memory_props: AdapterMemoryProperties,
    inner: Arc<Mutex<GpAllocatorInner>>,
}

impl GeneralPurposeAllocator {
    pub fn new(device: Device, memory_props: AdapterMemoryProperties) -> Self {
        Self {
            device,
            memory_props,
            inner: Arc::new(Mutex::new(GpAllocatorInner {
                pools: HashMap::new(),
            })),
        }
    }

    pub fn create_buffer(
        &self,
        size: NonZeroU64,
        usage: BufferUsage,
        flags: UsageFlags,
    ) -> BufferAlloc {
        let mut buffer = self.device.create_buffer(size, usage);
        let req = self.device.buffer_memory_requirements(&buffer);
        let memory = self.alloc(req, flags);
        unsafe {
            self.device.bind_buffer_memory(&mut buffer, memory.memory());
        }

        // The allocator may return a region bigger than requested.
        // In this case we simply ignore all memory exceeding the
        // requested allocation size.
        debug_assert!(memory.region.size >= size.get() as usize);

        BufferAlloc {
            buffer,
            memory,
            size: size.get() as usize,
        }
    }

    pub fn create_texture(
        &self,
        descriptor: &TextureDescriptor,
        flags: UsageFlags,
    ) -> TextureAlloc {
        let mut texture = self.device.create_texture(descriptor);
        let req = self.device.image_memory_requirements(&texture);
        let memory = self.alloc(req, flags);
        unsafe {
            self.device
                .bind_texture_memory(&mut texture, memory.memory());
        }

        TextureAlloc { texture, memory }
    }

    pub fn alloc(&self, mut req: MemoryRequirements, flags: UsageFlags) -> DeviceMemoryRegion {
        let inner = &mut *self.inner.lock();

        let host_visible = flags.contains(UsageFlags::HOST_VISIBLE);
        if host_visible {
            // Only `HOST_VISIBLE` memory is usable for the allocation.
            req.memory_types.retain(|id| {
                let mem_typ = self.memory_props.types[*id as usize];
                mem_typ.flags.contains(MemoryTypeFlags::HOST_VISIBLE)
            });
        } else {
            // All memory types are usable for this allocation, but since
            // host access is not requested we prefer memory types that are
            // "closer" to the GPU.
            req.memory_types.sort_by(|a, b| {
                let a = self.memory_props.types[*a as usize];
                let b = self.memory_props.types[*b as usize];

                // Preference as follows:
                // 1. Memory that is exactly `DEVICE_LOCAL` and nothing else.
                // 2. Memory that is `DEVICE_LOCAL` with other flags (probably `HOST_VISIBLE`).
                // 3. Memory that is not `HOST_COHERENT`.
                // 4. All remaining memory that is probably System RAM and snail pace.
                let prefs = &[
                    |flags: MemoryTypeFlags| flags == MemoryTypeFlags::DEVICE_LOCAL,
                    |flags: MemoryTypeFlags| flags.contains(MemoryTypeFlags::DEVICE_LOCAL),
                    |flags: MemoryTypeFlags| !flags.contains(MemoryTypeFlags::HOST_COHERENT),
                ];

                let weight_a = prefs
                    .iter()
                    .enumerate()
                    .map(|(index, f)| u8::from(f(a.flags)) << (prefs.len() - index))
                    .sum::<u8>();
                let weight_b = prefs
                    .iter()
                    .enumerate()
                    .map(|(index, f)| u8::from(f(b.flags)) << (prefs.len() - index))
                    .sum::<u8>();

                // Highest first
                weight_b.cmp(&weight_a)
            });
        }

        for mem_typ in req.memory_types {
            // Note: entry API does not work since we need to borrow inner
            // multiple times, so we use contains_key and get instead.
            if !inner.pools.contains_key(&mem_typ) {
                let size = core::cmp::max(MIN_SIZE, req.size.checked_next_power_of_two().unwrap());
                let mut memory = self.device.allocate_memory(size, mem_typ);
                let memory_host_ptr = if host_visible {
                    unsafe { memory.map(..).as_mut_ptr() }
                } else {
                    core::ptr::null_mut()
                };
                let mut blocks = Slab::new();
                let last = blocks.insert(Block {
                    memory: Arc::new(memory),
                    allocator: BuddyAllocator::new(Region {
                        offset: 0,
                        size: MIN_SIZE.get() as usize,
                    }),
                    num_allocs: 0,
                    size,
                    memory_host_ptr,
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
                    memory_host_ptr: block.memory_host_ptr,
                };
            }

            let prev_size = pool.blocks[pool.last].size;
            let new_size = core::cmp::max(
                prev_size.saturating_mul(GROWTH_FACTOR),
                req.size.checked_next_power_of_two().unwrap(),
            );
            let mut memory = self.device.allocate_memory(new_size, mem_typ);

            let memory_host_ptr = if host_visible {
                unsafe { memory.map(..).as_mut_ptr() }
            } else {
                core::ptr::null_mut()
            };

            let block = Block {
                allocator: BuddyAllocator::new(Region {
                    offset: 0,
                    size: new_size.get() as usize,
                }),
                memory: Arc::new(memory),
                num_allocs: 0,
                size: new_size,
                memory_host_ptr,
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
                memory_host_ptr: block.memory_host_ptr,
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
    /// Pointer to host memory if the allocation is host visible.
    /// We always use persistent mapping.
    memory_host_ptr: *mut u8,
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
    /// Pointer to host mapped memory. This pointer starts at memory, not at this region.
    memory_host_ptr: *mut u8,
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

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct UsageFlags: u32 {
        const HOST_VISIBLE = 1 << 0;
    }
}

pub struct BufferAlloc {
    buffer: Buffer,
    memory: DeviceMemoryRegion,
    size: usize,
}

impl BufferAlloc {
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub unsafe fn map(&mut self) -> &mut [u8] {
        assert!(!self.memory.memory_host_ptr.is_null());

        unsafe {
            let ptr = self.memory.memory_host_ptr.add(self.memory.region.offset);
            let len = self.size;
            core::slice::from_raw_parts_mut(ptr, len)
        }
    }
}

pub struct TextureAlloc {
    texture: Texture,
    memory: DeviceMemoryRegion,
}

impl TextureAlloc {
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

pub struct MappedMemory<'a> {
    region: DeviceMemoryRegion,
    memory: &'a mut [u8],
}

impl<'a> Deref for MappedMemory<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.memory
    }
}

impl<'a> DerefMut for MappedMemory<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.memory
    }
}

impl<'a> Drop for MappedMemory<'a> {
    fn drop(&mut self) {}
}
