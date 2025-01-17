use std::collections::HashMap;
use std::mem::transmute;
use std::num::NonZeroU32;

use game_tracing::trace_span;
use slab::Slab;

use super::vulkan::{DescriptorPool, DescriptorSet, DescriptorSetLayout, Device, Error};
use super::{DescriptorPoolDescriptor, DescriptorType};

const MIN_POOL_SIZE: NonZeroU32 = NonZeroU32::new(1).unwrap();
const MAX_POOL_SIZE: NonZeroU32 = NonZeroU32::new(32).unwrap();
const GROWTH_FACTOR: NonZeroU32 = NonZeroU32::new(2).unwrap();

pub struct AllocatedDescriptorSet {
    set: DescriptorSet<'static>,
    bucket: DescriptorSetResourceCount,
    pool: usize,
}

impl AllocatedDescriptorSet {
    pub fn raw(&self) -> &DescriptorSet<'static> {
        &self.set
    }

    pub fn raw_mut(&mut self) -> &mut DescriptorSet<'static> {
        &mut self.set
    }
}

pub struct DescriptorSetAllocator {
    device: Device,
    buckets: HashMap<DescriptorSetResourceCount, DescriptorPoolBucket>,
}

impl DescriptorSetAllocator {
    pub fn new(device: Device) -> Self {
        Self {
            device,
            buckets: HashMap::new(),
        }
    }

    pub unsafe fn alloc(
        &mut self,
        layout: &DescriptorSetLayout,
    ) -> Result<AllocatedDescriptorSet, Error> {
        let _span = trace_span!("DescriptorSetAllocator::alloc").entered();

        let mut count = DescriptorSetResourceCount::default();

        for binding in layout.bindings() {
            match binding.kind {
                DescriptorType::Uniform => {
                    count.uniform_buffers += 1;
                }
                DescriptorType::Storage => {
                    count.storage_buffers += 1;
                }
                DescriptorType::Sampler => {
                    count.samplers += 1;
                }
                DescriptorType::Texture => {
                    count.textures += 1;
                }
            }
        }

        let bucket = self
            .buckets
            .entry(count)
            .or_insert_with(|| DescriptorPoolBucket::new());

        match unsafe { bucket.alloc(&self.device, &count, layout) } {
            Ok((set, pool)) => {
                let set = unsafe { transmute::<DescriptorSet<'_>, DescriptorSet<'static>>(set) };
                Ok(AllocatedDescriptorSet {
                    set,
                    bucket: count,
                    pool,
                })
            }
            Err(err) => {
                // `alloc` should handle out-of-pool-memory errors internally.
                debug_assert_ne!(err, Error::OutOfPoolMemory);
                Err(err)
            }
        }
    }

    pub unsafe fn dealloc(&mut self, descriptor_set: AllocatedDescriptorSet) {
        let _span = trace_span!("DescriptorSetAllocator::dealloc").entered();

        let bucket = self.buckets.get_mut(&descriptor_set.bucket).unwrap();

        unsafe {
            bucket.dealloc(descriptor_set.pool);
        }
    }
}

struct DescriptorPoolBucket {
    pools: Slab<Pool>,
    next_pool_size: NonZeroU32,
}

impl DescriptorPoolBucket {
    fn new() -> Self {
        Self {
            pools: Slab::new(),
            next_pool_size: MIN_POOL_SIZE,
        }
    }

    /// Allocates a new [`DescriptorSet`] from the bucket.
    ///
    /// The `usize` value must be given to [`dealloc`] when the set is deallocated.
    ///
    /// # Safety
    ///
    /// The returned [`DescriptorSet`] must be dropped before the provided [`Device`] and `self`.
    unsafe fn alloc(
        &mut self,
        device: &Device,
        count: &DescriptorSetResourceCount,
        layout: &DescriptorSetLayout,
    ) -> Result<(DescriptorSet<'_>, usize), Error> {
        for (key, pool) in self.pools.iter_mut() {
            if pool.free == 0 {
                continue;
            }

            let set = match pool.pool.create_descriptor_set(layout) {
                Ok(set) => set,
                // The pool may still return out of pool memory errors,
                // even after we have checked that it should have
                // enough memory.
                Err(Error::OutOfPoolMemory) => continue,
                Err(err) => return Err(err),
            };

            pool.free -= 1;
            pool.allocated += 1;

            // Drop the lifetime.
            let set = unsafe { transmute::<DescriptorSet<'_>, DescriptorSet<'_>>(set) };
            return Ok((set, key));
        }

        let pool_size = self.next_pool_size;
        self.next_pool_size =
            (self.next_pool_size.saturating_mul(GROWTH_FACTOR)).min(MAX_POOL_SIZE);
        let pool = device.create_descriptor_pool(&DescriptorPoolDescriptor {
            max_sets: pool_size,
            max_uniform_buffers: count.uniform_buffers * pool_size.get(),
            max_storage_buffers: count.storage_buffers * pool_size.get(),
            max_samplers: count.samplers * pool_size.get(),
            max_sampled_images: count.textures * pool_size.get(),
        });
        // Drop the lifetime of the pool. The caller guarantees that `self` outlives
        // the passed `device` handle.
        let pool = unsafe { transmute::<DescriptorPool<'_>, DescriptorPool<'static>>(pool) };
        let pool = Pool {
            pool,
            // We are immediately allocating a set from the pool.
            free: pool_size.get() - 1,
            allocated: 1,
        };

        let key = self.pools.insert(pool);
        let set = self
            .pools
            .get_mut(key)
            .unwrap()
            .pool
            .create_descriptor_set(layout)?;
        Ok((set, key))
    }

    unsafe fn dealloc(&mut self, pool_index: usize) {
        let pool = self.pools.get_mut(pool_index).unwrap();

        pool.allocated -= 1;
        if pool.free == 0 && pool.allocated == 0 {
            self.pools.remove(pool_index);
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct DescriptorSetResourceCount {
    uniform_buffers: u32,
    storage_buffers: u32,
    samplers: u32,
    textures: u32,
}

struct Pool {
    pool: DescriptorPool<'static>,
    free: u32,
    allocated: u32,
}
