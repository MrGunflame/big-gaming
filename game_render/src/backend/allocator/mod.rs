mod buddy;
mod bump;
mod free_list;

pub use buddy::BuddyAllocator;
use game_tracing::trace_span;

use std::alloc::Layout;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bitflags::bitflags;
use thiserror::Error;

use super::vulkan::{self, Device, DeviceMemory};
use super::{AdapterMemoryProperties, DedicatedResource};

#[derive(Clone, Debug, Error)]
pub enum AllocError {
    #[error("heap is full")]
    HeapFull,
    #[error("allocation exceeds max allocation size")]
    AllocationTooBig,
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
        dedicated_for: Option<DedicatedResource<'_>>,
    ) -> Result<MemoryAllocation, AllocError> {
        let _span = trace_span!("MemoryManager::allocate").entered();

        if size > self.inner.properties.max_allocation_size {
            return Err(AllocError::AllocationTooBig);
        }

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

        let memory = match self
            .inner
            .device
            .allocate_memory(size, memory_type, dedicated_for)
        {
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
        let _span = trace_span!("MemoryManager::deallocate").entered();

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
    fn new(region: Region) -> Self;

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

/// An allocator that supports growth of its underlying memory without invalidating allocations.
pub trait GrowableAllocator: Allocator {
    /// Grows the allocator to the new size.
    ///
    /// # Safety
    ///
    /// - The offset of the new region must be same as the old region.
    /// - The size of the new region must be greater than or equal to the size of the old region.
    unsafe fn grow(&mut self, new_region: Region);
}

#[derive(Copy, Clone, Debug)]
pub struct Region {
    pub offset: usize,
    pub size: usize,
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

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct UsageFlags: u32 {
        const HOST_VISIBLE = 1 << 0;
    }
}
