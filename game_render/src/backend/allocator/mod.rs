mod block;
mod buddy;
mod bump;

use std::alloc::Layout;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bitflags::bitflags;
pub use block::BlockAllocator;
pub use buddy::BuddyAllocator;
use game_tracing::trace_span;
use parking_lot::Mutex;
use slab::Slab;
use thiserror::Error;

use crate::backend::MemoryTypeFlags;

use super::vulkan::{self, Buffer, Device, DeviceMemory, DeviceMemorySlice, Texture};
use super::{
    AdapterMemoryProperties, BufferUsage, BufferView, MemoryRequirements, TextureDescriptor,
};

#[derive(Clone, Debug, Error)]
pub enum AllocError {
    #[error("heap is full")]
    HeapFull,
    #[error(transparent)]
    Other(vulkan::Error),
}

#[derive(Debug)]
struct Heap {
    size: u64,
    used: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct MemoryManager {
    inner: Arc<MemoryManagerInner>,
}

impl MemoryManager {
    pub fn new(device: Device, properties: AdapterMemoryProperties) -> Self {
        let heaps = properties
            .heaps
            .iter()
            .map(|heap| Heap {
                size: heap.size,
                used: AtomicU64::new(0),
            })
            .collect();

        Self {
            inner: Arc::new(MemoryManagerInner {
                device,
                heaps,
                properties,
            }),
        }
    }

    pub fn allocate(
        &self,
        size: NonZeroU64,
        memory_type: u32,
    ) -> Result<MemoryAllocation, AllocError> {
        let typ = &self.inner.properties.types[memory_type as usize];
        let heap = &self.inner.heaps[typ.heap as usize];

        if let Err(_) = heap
            .used
            .fetch_update(Ordering::Release, Ordering::Acquire, |used| {
                used.checked_add(size.get())
                    .filter(|used| *used <= heap.size)
            })
        {
            return Err(AllocError::HeapFull);
        }

        let memory = match self.inner.device.allocate_memory(size, memory_type) {
            Ok(memory) => memory,
            Err(err) => {
                // If the allocation fails it must not contribute to the
                // heap usage.
                heap.used.fetch_sub(size.get(), Ordering::Release);
                return Err(AllocError::Other(err));
            }
        };

        Ok(MemoryAllocation {
            manager: self.clone(),
            memory,
            size,
            memory_type,
        })
    }

    fn deallocate(&self, size: NonZeroU64, memory_type: u32) {
        let typ = &self.inner.properties.types[memory_type as usize];
        let heap = &self.inner.heaps[typ.heap as usize];
        heap.used.fetch_sub(size.get(), Ordering::Release);
    }

    pub fn properties(&self) -> &AdapterMemoryProperties {
        &self.inner.properties
    }
}

#[derive(Debug)]
struct MemoryManagerInner {
    device: Device,
    heaps: Box<[Heap]>,
    properties: AdapterMemoryProperties,
}

#[derive(Debug)]
pub struct MemoryAllocation {
    manager: MemoryManager,
    memory: DeviceMemory,
    size: NonZeroU64,
    memory_type: u32,
}

impl Deref for MemoryAllocation {
    type Target = DeviceMemory;

    fn deref(&self) -> &Self::Target {
        &self.memory
    }
}

impl DerefMut for MemoryAllocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.memory
    }
}

impl Drop for MemoryAllocation {
    fn drop(&mut self) {
        self.manager.deallocate(self.size, self.memory_type);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllocationError {
    OutOfMemory,
}

pub trait Allocator {
    /// Allocates a new memory region using this allocator.
    ///
    /// The returned [`Region`] may be bigger than the size requested. Returns `None` if the
    /// allocator does not have enough free memory to allocate the given [`Layout`].
    fn alloc(&mut self, layout: Layout) -> Option<Region>;

    /// Deallocates a previously allocated memory region.
    ///
    /// # Safety
    ///
    /// The [`Region`] must have been previously been returned by [`alloc`] on this allocator
    /// instance. Every [`Region`] returned by [`alloc`] must only be used in `dealloc` once.
    ///
    /// [`alloc`]: Allocator::alloc
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

const MIN_SIZE: NonZeroU64 = NonZeroU64::new(1_048_576).unwrap();
const MAX_SIZE: NonZeroU64 = NonZeroU64::new(4_294_967_296).unwrap();
const GROWTH_FACTOR: NonZeroU64 = NonZeroU64::new(2).unwrap();

#[derive(Clone, Debug)]
pub struct GeneralPurposeAllocator {
    device: Device,
    manager: MemoryManager,
    inner: Arc<Mutex<GpAllocatorInner>>,
}

impl GeneralPurposeAllocator {
    pub fn new(device: Device, manager: MemoryManager) -> Self {
        Self {
            device,
            manager,
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
        let mut buffer = self.device.create_buffer(size, usage).unwrap();
        let req = self.device.buffer_memory_requirements(&buffer);
        let memory = self.alloc(req, flags);
        unsafe {
            self.device
                .bind_buffer_memory(&mut buffer, memory.memory())
                .unwrap();
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
        let mut texture = self.device.create_texture(descriptor).unwrap();
        let req = self.device.image_memory_requirements(&texture);
        let memory = self.alloc(req, flags);
        unsafe {
            self.device
                .bind_texture_memory(&mut texture, memory.memory())
                .unwrap();
        }

        TextureAlloc { texture, memory }
    }

    pub fn alloc(&self, mut req: MemoryRequirements, flags: UsageFlags) -> DeviceMemoryRegion {
        let _span = trace_span!("GeneralPurposeAllocator::alloc").entered();

        let inner = &mut *self.inner.lock();

        let host_visible = flags.contains(UsageFlags::HOST_VISIBLE);
        if host_visible {
            // Only `HOST_VISIBLE` memory is usable for the allocation.
            req.memory_types.retain(|id| {
                let mem_typ = self.manager.properties().types[*id as usize];
                mem_typ.flags.contains(MemoryTypeFlags::HOST_VISIBLE)
            });
        } else {
            // All memory types are usable for this allocation, but since
            // host access is not requested we prefer memory types that are
            // "closer" to the GPU.
            req.memory_types.sort_by(|a, b| {
                let a = self.manager.properties().types[*a as usize];
                let b = self.manager.properties().types[*b as usize];

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

        for &mem_typ in &req.memory_types {
            let host_visible = self.manager.properties().types[mem_typ as usize]
                .flags
                .contains(MemoryTypeFlags::HOST_VISIBLE);

            let pool = inner
                .pools
                .entry(mem_typ)
                .or_insert_with(|| Pool::new(mem_typ));

            let allocation = pool.alloc(&self.manager, &req, host_visible).unwrap();
            return DeviceMemoryRegion {
                allocator: self.clone(),
                memory: allocation.memory.clone(),
                region: allocation.region,
                memory_type: allocation.memory.memory_type,
                memory_host_ptr: allocation.memory_host_ptr,
                strategy: allocation.strategy,
            };
        }

        todo!()
    }

    unsafe fn dealloc(&self, mem_typ: u32, strategy: Strategy, region: Region) {
        let _span = trace_span!("GeneralPurposeAllocator::dealloc").entered();

        let mut inner = self.inner.lock();

        let pool = inner.pools.get_mut(&mem_typ).unwrap();

        unsafe {
            pool.dealloc(strategy, region);
        }

        if pool.is_empty() {
            inner.pools.remove(&mem_typ);
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
    next_block_size: NonZeroU64,
    memory_type: u32,
    dedicated_allocs: usize,
}

impl Pool {
    fn new(memory_type: u32) -> Self {
        Self {
            blocks: Slab::new(),
            next_block_size: MIN_SIZE,
            memory_type,
            dedicated_allocs: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.dedicated_allocs == 0
    }

    fn alloc(
        &mut self,
        manager: &MemoryManager,
        req: &MemoryRequirements,
        host_visible: bool,
    ) -> Result<PoolAllocation, AllocError> {
        if req.size.get() > MAX_SIZE.get() {
            let mut memory = manager.allocate(req.size, self.memory_type)?;
            let memory_host_ptr = if host_visible {
                unsafe { memory.map(..).as_mut_ptr() }
            } else {
                core::ptr::null_mut()
            };

            self.dedicated_allocs += 1;

            return Ok(PoolAllocation {
                region: Region::new(0, memory.size.get() as usize),
                memory: Arc::new(memory),
                memory_host_ptr,
                strategy: Strategy::Dedicated,
            });
        }

        for (block_index, block) in &mut self.blocks {
            let Some(region) = block.alloc(req.size, req.align) else {
                continue;
            };

            return Ok(PoolAllocation {
                memory: block.memory.clone(),
                region,
                memory_host_ptr: block.memory_host_ptr,
                strategy: Strategy::Block { block_index },
            });
        }

        let block_size = core::cmp::max(
            self.next_block_size,
            req.size.checked_next_power_of_two().unwrap(),
        );
        self.next_block_size = block_size.saturating_mul(GROWTH_FACTOR).min(MAX_SIZE);

        let mut memory = manager.allocate(block_size, self.memory_type)?;
        let memory_host_ptr = if host_visible {
            unsafe { memory.map(..).as_mut_ptr() }
        } else {
            core::ptr::null_mut()
        };

        let block_index = self.blocks.insert(Block {
            memory: Arc::new(memory),
            allocator: BuddyAllocator::new(Region {
                offset: 0,
                size: block_size.get() as usize,
            }),
            num_allocs: 0,
            size: block_size,
            memory_host_ptr,
        });
        let block = &mut self.blocks[block_index];

        let region = block.alloc(req.size, req.align).unwrap();

        Ok(PoolAllocation {
            memory: block.memory.clone(),
            region,
            memory_host_ptr: block.memory_host_ptr,
            strategy: Strategy::Block { block_index },
        })
    }

    unsafe fn dealloc(&mut self, strategy: Strategy, region: Region) {
        match strategy {
            Strategy::Block { block_index } => {
                let block = &mut self.blocks[block_index];

                unsafe {
                    block.allocator.dealloc(region);
                }

                block.num_allocs -= 1;
                if block.num_allocs != 0 {
                    return;
                }

                self.blocks.remove(block_index);
            }
            Strategy::Dedicated => {
                self.dedicated_allocs -= 1;
            }
        }
    }
}

#[derive(Debug)]
struct PoolAllocation {
    memory: Arc<MemoryAllocation>,
    region: Region,
    strategy: Strategy,
    memory_host_ptr: *mut u8,
}

#[derive(Copy, Clone, Debug)]
enum Strategy {
    Block { block_index: usize },
    Dedicated,
}

#[derive(Debug)]
struct Block {
    memory: Arc<MemoryAllocation>,
    allocator: BuddyAllocator,
    num_allocs: usize,
    size: NonZeroU64,
    /// Pointer to host memory if the allocation is host visible.
    /// We always use persistent mapping.
    memory_host_ptr: *mut u8,
}

// We lose the `Send` impl because of `memory_host_ptr`, but
// host-mapped memory is always send + sync.
unsafe impl Send for Block {}
unsafe impl Sync for Block {}

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
    memory: Arc<MemoryAllocation>,
    region: Region,
    memory_type: u32,
    /// Pointer to host mapped memory. This pointer starts at memory, not at this region.
    memory_host_ptr: *mut u8,
    strategy: Strategy,
}

// We lose the `Send` impl because of `memory_host_ptr`, but
// host-mapped memory is always send + sync.
unsafe impl Send for DeviceMemoryRegion {}
unsafe impl Sync for DeviceMemoryRegion {}

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
                .dealloc(self.memory_type, self.strategy, self.region);
        }
    }
}

impl Debug for DeviceMemoryRegion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(DeviceMemoryRegion))
            .field("memory", &self.memory)
            .field("memory_type", &self.memory_type)
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct UsageFlags: u32 {
        const HOST_VISIBLE = 1 << 0;
    }
}

#[derive(Debug)]
pub struct BufferAlloc {
    buffer: Buffer,
    memory: DeviceMemoryRegion,
    size: usize,
}

impl BufferAlloc {
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn size(&self) -> u64 {
        self.size as u64
    }

    pub fn buffer_view(&self) -> BufferView<'_> {
        self.buffer.slice(..)
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

#[derive(Debug)]
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
