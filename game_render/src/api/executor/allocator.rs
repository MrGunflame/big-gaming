use std::alloc::Layout;
use std::num::NonZeroU64;
use std::ptr::NonNull;
use std::sync::Arc;

use game_tracing::trace_span;
use hashbrown::HashMap;
use slab::Slab;
use thiserror::Error;

use crate::api::executor::{BufferAlloc, TextureAlloc};
use crate::api::BufferDescriptor;
use crate::backend::allocator::{
    AllocError, Allocator, BuddyAllocator, MemoryAllocation, MemoryManager, Region, UsageFlags,
};
use crate::backend::vulkan::{Device, DeviceMemorySlice};
use crate::backend::{DedicatedResource, MemoryRequirements, MemoryTypeFlags, TextureDescriptor};
use crate::statistics::{AllocationKind, MemoryAlloc, MemoryBlock, Statistics};

const MIN_PAGE_SIZE: NonZeroU64 = NonZeroU64::new(1 << 20).unwrap();
const GROWTH_FACTOR: NonZeroU64 = NonZeroU64::new(2).unwrap();

#[derive(Clone, Debug, Error)]
#[error("allocation {req:?} with flags {flags:?} failed")]
pub struct Error {
    req: MemoryRequirements,
    flags: UsageFlags,
}

#[derive(Debug)]
pub struct MemoryAllocator {
    device: Device,
    manager: MemoryManager,
    pages: HashMap<u32, PagePool>,
    statistics: Arc<Statistics>,
}

impl MemoryAllocator {
    pub fn new(device: Device, manager: MemoryManager, statistics: Arc<Statistics>) -> Self {
        let memory_type_count = manager.properties().types.len();

        Self {
            device,
            manager,
            pages: HashMap::with_capacity(memory_type_count),
            statistics,
        }
    }

    pub fn create_buffer(&mut self, descriptor: &BufferDescriptor) -> BufferAlloc {
        let size = NonZeroU64::new(descriptor.size).unwrap();

        let mut buffer = self.device.create_buffer(size, descriptor.usage).unwrap();
        let req = self.device.buffer_memory_requirements(&buffer);

        let allocation = self
            .alloc(
                req,
                descriptor.flags,
                Some(DedicatedResource::Buffer(&buffer)),
            )
            .unwrap();

        unsafe {
            self.device
                .bind_buffer_memory(&mut buffer, allocation.memory())
                .unwrap();
        }

        let mut stats = self.statistics.memory.write();
        let block = &mut stats.blocks[allocation.stats_block_index];
        block.used += allocation.size as u64;
        block.allocs.insert(
            allocation.region.offset as u64,
            MemoryAlloc {
                offset: allocation.region.offset as u64,
                size: allocation.size as u64,
                kind: AllocationKind::Buffer,
            },
        );

        BufferAlloc { buffer, allocation }
    }

    pub fn create_texture(
        &mut self,
        descriptor: &TextureDescriptor,
        flags: UsageFlags,
    ) -> TextureAlloc {
        let mut texture = self.device.create_texture(descriptor).unwrap();
        let req = self.device.image_memory_requirements(&texture);

        let allocation = self
            .alloc(req, flags, Some(DedicatedResource::Texture(&texture)))
            .unwrap();

        unsafe {
            self.device
                .bind_texture_memory(&mut texture, allocation.memory())
                .unwrap();
        }

        let mut stats = self.statistics.memory.write();
        let block = &mut stats.blocks[allocation.stats_block_index];
        block.used += allocation.size as u64;
        block.allocs.insert(
            allocation.region.offset as u64,
            MemoryAlloc {
                offset: allocation.region.offset as u64,
                size: allocation.size as u64,
                kind: AllocationKind::Texture,
            },
        );

        TextureAlloc {
            texture,
            allocation,
        }
    }

    fn alloc(
        &mut self,
        mut req: MemoryRequirements,
        flags: UsageFlags,
        dedicated_for: Option<DedicatedResource<'_>>,
    ) -> Result<Allocation, Error> {
        let _span = trace_span!("MemoryAllocator::alloc").entered();

        let props = self.manager.properties();

        let use_dedicated =
            req.dedicated.is_preferred_or_required() || req.size > props.max_allocation_size;

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

        // For dedicated allocations we try all memory types with a dedicated allocation.
        // If all of them fail and a dedicated allocation is only preferred and not
        // required we try to fall back to a regular allocation.
        if use_dedicated {
            for &memory_type in &req.memory_types {
                let Ok(mut memory) = self
                    .manager
                    .allocate(req.size, memory_type, dedicated_for)
                    .inspect_err(|err| {
                        tracing::error!(
                            "dedicated allocation of {:?} for memory type {:?} failed: {}",
                            req,
                            memory_type,
                            err,
                        );
                    })
                else {
                    continue;
                };

                let ptr = host_visible.then(|| memory.map().unwrap());

                let device_local = props.types[memory_type as usize]
                    .flags
                    .contains(MemoryTypeFlags::DEVICE_LOCAL);

                let stats_block_index = self.statistics.memory.write().blocks.insert(MemoryBlock {
                    size: req.size.get(),
                    used: 0,
                    allocs: HashMap::new(),
                    dedicated: true,
                    device_local,
                    host_visible,
                });

                return Ok(Allocation {
                    memory: Arc::new(memory),
                    strategy: Strategy::Dedicated,
                    region: Region {
                        offset: 0,
                        size: req.size.get() as usize,
                    },
                    ptr,
                    flags: props.types[memory_type as usize].flags,
                    stats_block_index,
                    size: req.size.get() as usize,
                });
            }

            if req.dedicated.is_required() || req.size > props.max_allocation_size {
                return Err(Error { req, flags });
            }
        }

        for &memory_type in &req.memory_types {
            let pool = self.pages.entry(memory_type).or_insert_with(|| PagePool {
                pages: Slab::new(),
                memory_type,
                next_page_size: MIN_PAGE_SIZE,
            });

            let device_local = props.types[memory_type as usize]
                .flags
                .contains(MemoryTypeFlags::DEVICE_LOCAL);

            let Ok(allocation) = pool
                .alloc(
                    &self.manager,
                    &req,
                    host_visible,
                    device_local,
                    &self.statistics,
                )
                .inspect_err(|err| {
                    tracing::error!(
                        "allocation of {:?} for memory type {} failed: {}",
                        req,
                        memory_type,
                        err
                    );
                })
            else {
                continue;
            };

            return Ok(Allocation {
                memory: allocation.memory,
                strategy: Strategy::PagePool {
                    memory_type,
                    page_index: allocation.page_index,
                },
                region: allocation.region,
                ptr: allocation.ptr,
                flags: props.types[memory_type as usize].flags,
                stats_block_index: allocation.stats_block_index,
                size: req.size.get() as usize,
            });
        }

        Err(Error { req, flags })
    }

    pub unsafe fn dealloc(&mut self, allocation: Allocation) {
        let _span = trace_span!("MemoryAllocator::dealloc").entered();

        {
            let mut stats = self.statistics.memory.write();
            let block = &mut stats.blocks[allocation.stats_block_index];
            block.used -= allocation.size as u64;
            block.allocs.remove(&(allocation.region.offset as u64));
        }

        match allocation.strategy {
            Strategy::Dedicated => {
                self.statistics
                    .memory
                    .write()
                    .blocks
                    .remove(allocation.stats_block_index);
            }

            Strategy::PagePool {
                memory_type,
                page_index,
            } => {
                // SAFETY: The caller guarantees that the allocation is valid.
                let pool = unsafe { self.pages.get_mut(&memory_type).unwrap_unchecked() };

                // SAFETY: The caller guarantees that the allocation is valid.
                unsafe {
                    pool.dealloc(&self.statistics, page_index, allocation.region);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Allocation {
    memory: Arc<MemoryAllocation>,
    strategy: Strategy,
    region: Region,
    ptr: Option<NonNull<u8>>,
    flags: MemoryTypeFlags,
    stats_block_index: usize,
    size: usize,
}

unsafe impl Send for Allocation {}
unsafe impl Sync for Allocation {}

impl Allocation {
    pub fn memory(&self) -> DeviceMemorySlice<'_> {
        self.memory
            .slice(self.region.offset as u64..self.region.offset as u64 + self.region.size as u64)
    }

    pub fn flags(&self) -> MemoryTypeFlags {
        self.flags
    }

    /// Returns a slice to the allocated memory.
    pub fn as_bytes(&mut self) -> &mut [u8] {
        let ptr = self
            .ptr
            .expect("cannot call as_bytes on non host-visible memory");

        unsafe {
            let data = ptr.as_ptr();
            let len = self.size;
            core::slice::from_raw_parts_mut(data, len)
        }
    }

    // FIXME: This is a terrible API.
    // The caller should only have the guarantee that its own memory
    // is not mapped, not that all memories are not mapped.
    // It couples the state of the buffer with all buffers.
    /// Flushes the memory of this buffer.
    ///
    /// # Safety
    ///
    /// No memory (of any `BufferAlloc`) must currently be mapped using [`map`].
    ///
    /// [`map`]: Self::map
    pub unsafe fn flush(&mut self) {
        self.memory.flush().unwrap();
    }
}

/// Allocation strategy.
#[derive(Copy, Clone, Debug)]
enum Strategy {
    Dedicated,
    PagePool { memory_type: u32, page_index: usize },
}

#[derive(Debug)]
struct PagePool {
    pages: Slab<Page>,
    memory_type: u32,
    next_page_size: NonZeroU64,
}

impl PagePool {
    fn alloc(
        &mut self,
        manager: &MemoryManager,
        req: &MemoryRequirements,
        host_visible: bool,
        device_local: bool,
        stats: &Statistics,
    ) -> Result<PagePoolAllocation, AllocError> {
        let props = manager.properties();

        debug_assert!(props.max_allocation_size >= req.size);
        debug_assert!(!req.dedicated.is_required());

        let layout =
            Layout::from_size_align(req.size.get() as usize, req.align.get() as usize).unwrap();

        for (page_index, page) in &mut self.pages {
            let Some(region) = page.allocator.alloc(layout) else {
                continue;
            };

            let ptr = page.ptr.map(|ptr| unsafe { ptr.add(region.offset) });

            page.num_allocs += 1;
            return Ok(PagePoolAllocation {
                memory: page.memory.clone(),
                page_index,
                region,
                ptr,
                stats_block_index: page.stats_block_index,
            });
        }

        let size_padded = NonZeroU64::new(layout.pad_to_align().size() as u64).unwrap();
        let page_size = self
            .next_page_size
            .max(size_padded)
            .checked_next_power_of_two()
            .unwrap();

        let max_size_pow2 = prev_power_of_two(props.max_allocation_size);
        self.next_page_size = page_size.saturating_mul(GROWTH_FACTOR).min(max_size_pow2);

        let mut memory = manager.allocate(page_size, self.memory_type, None)?;
        let ptr = host_visible.then(|| memory.map().unwrap());

        let stats_block_index = stats.memory.write().blocks.insert(MemoryBlock {
            size: page_size.get(),
            used: 0,
            allocs: HashMap::new(),
            dedicated: false,
            device_local,
            host_visible,
        });

        let page_index = self.pages.insert(Page {
            memory: Arc::new(memory),
            allocator: BuddyAllocator::new(Region::new(0, page_size.get() as usize)),
            num_allocs: 0,
            ptr,
            stats_block_index,
        });
        let page = &mut self.pages[page_index];

        let region = page.allocator.alloc(layout).unwrap();
        let ptr = page.ptr.map(|ptr| unsafe { ptr.add(region.offset) });
        page.num_allocs += 1;

        Ok(PagePoolAllocation {
            memory: page.memory.clone(),
            page_index,
            region,
            ptr,
            stats_block_index: page.stats_block_index,
        })
    }

    /// Deallocates an allocation made in this `PagePool`.
    ///
    /// # Safety
    ///
    /// The given `page_index` and `region` must have previously been returned by [`alloc`]. Every
    /// allocation must only be deallocated once.
    ///
    /// [`alloc`]: Self::alloc
    unsafe fn dealloc(&mut self, stats: &Statistics, page_index: usize, region: Region) {
        // SAFETY: The caller guarantees that the index is valid.
        let page = unsafe { self.pages.get_unchecked_mut(page_index) };

        // SAFETY: The caller guarantees that the region was previously
        // allocated in this page.
        unsafe {
            page.allocator.dealloc(region);
        }

        debug_assert_ne!(page.num_allocs, 0);
        page.num_allocs -= 1;

        if page.num_allocs == 0 {
            stats.memory.write().blocks.remove(page.stats_block_index);

            self.pages.remove(page_index);
        }
    }
}

#[derive(Debug)]
struct Page {
    memory: Arc<MemoryAllocation>,
    allocator: BuddyAllocator,
    num_allocs: usize,
    ptr: Option<NonNull<u8>>,
    stats_block_index: usize,
}

unsafe impl Send for Page {}
unsafe impl Sync for Page {}

#[derive(Clone, Debug)]
struct PagePoolAllocation {
    memory: Arc<MemoryAllocation>,
    page_index: usize,
    region: Region,
    ptr: Option<NonNull<u8>>,
    stats_block_index: usize,
}

unsafe impl Send for PagePoolAllocation {}
unsafe impl Sync for PagePoolAllocation {}

/// Returns the previous power of two value.
fn prev_power_of_two(x: NonZeroU64) -> NonZeroU64 {
    // Note that this can never be zero, since 1 << n is always greater
    // than 0.
    // The subtraction cannot overflow, since `leading_zeroes` will never
    // return 64, since `x` is never 0. As such 63 - n where n < 64 is ok.
    unsafe { NonZeroU64::new_unchecked(1 << (u64::BITS - 1) - x.leading_zeros()) }
}
