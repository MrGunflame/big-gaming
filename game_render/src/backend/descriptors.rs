use std::collections::HashMap;
use std::mem::transmute;
use std::num::NonZeroU32;

use game_tracing::trace_span;
use slab::Slab;

use super::vulkan::{DescriptorPool, DescriptorSet, DescriptorSetLayout, Device};
use super::{DescriptorPoolDescriptor, DescriptorType};

const MIN_POOL_SIZE: NonZeroU32 = NonZeroU32::new(1).unwrap();
const MAX_POOL_SIZE: NonZeroU32 = NonZeroU32::new(32).unwrap();

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

pub struct DescriptorSetAllocator<'a> {
    device: &'a Device<'a>,
    buckets: HashMap<DescriptorSetResourceCount, DescriptorPoolBucket>,
}

impl<'a> DescriptorSetAllocator<'a> {
    pub fn new(device: &'a Device<'a>) -> Self {
        Self {
            device,
            buckets: HashMap::new(),
        }
    }

    pub unsafe fn alloc(&mut self, layout: &DescriptorSetLayout<'_>) -> AllocatedDescriptorSet {
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
            }
        }

        let bucket = self
            .buckets
            .entry(count)
            .or_insert_with(|| DescriptorPoolBucket::new());

        let (set, pool) = unsafe { bucket.alloc(self.device, &count, layout) };
        let set = unsafe { transmute::<DescriptorSet<'_>, DescriptorSet<'static>>(set) };

        AllocatedDescriptorSet {
            set,
            bucket: count,
            pool,
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

    unsafe fn alloc(
        &mut self,
        device: &Device<'_>,
        count: &DescriptorSetResourceCount,
        layout: &DescriptorSetLayout<'_>,
    ) -> (DescriptorSet<'_>, usize) {
        {
            for (key, pool) in self.pools.iter_mut() {
                let set = pool.pool.create_descriptor_set(layout);
                pool.count += 1;
                // Drop the lifetime.
                let set = unsafe { transmute::<DescriptorSet<'_>, DescriptorSet<'_>>(set) };
                return (set, key);
            }
        }

        let pool_size = self.next_pool_size;
        self.next_pool_size = (self
            .next_pool_size
            .saturating_mul(NonZeroU32::new(2).unwrap()))
        .min(MAX_POOL_SIZE);
        let pool = device.create_descriptor_pool(&DescriptorPoolDescriptor {
            max_sets: pool_size,
            max_uniform_buffers: count.uniform_buffers * pool_size.get(),
            max_storage_buffers: count.storage_buffers * pool_size.get(),
        });
        // Drop the lifetime of the pool. The caller guarantees that `self` outlives
        // the passed `device` handle.
        let pool = unsafe { transmute::<DescriptorPool<'_>, DescriptorPool<'static>>(pool) };
        let pool = Pool { pool, count: 1 };

        let key = self.pools.insert(pool);
        let set = self
            .pools
            .get_mut(key)
            .unwrap()
            .pool
            .create_descriptor_set(layout);
        (set, key)
    }

    unsafe fn dealloc(&mut self, pool_index: usize) {
        let pool = self.pools.get_mut(pool_index).unwrap();
        pool.count -= 1;
        if pool.count == 0 {
            // The DescriptorPool is destroyed once the object
            // is dropped.
            self.pools.remove(pool_index);
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct DescriptorSetResourceCount {
    uniform_buffers: u32,
    storage_buffers: u32,
}

struct Pool {
    pool: DescriptorPool<'static>,
    /// Number of active descriptors in the pool.
    count: usize,
}
