use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use crate::backend::vulkan::{Device, QueryPool};
use crate::backend::{QueryKind, QueryPoolDescriptor};

#[derive(Debug)]
pub struct QueryPoolSet {
    device: Device,
    pools: Arc<Mutex<BTreeMap<NonZeroU32, Arc<QueryPool>>>>,
}

impl QueryPoolSet {
    pub fn new(device: Device) -> Self {
        Self {
            device,
            pools: Arc::default(),
        }
    }

    pub fn get(&self, count: NonZeroU32) -> ManagedQueryPool {
        {
            let mut pools = self.pools.lock();
            for (&size, _) in pools.range(count..) {
                let pool = pools.remove(&size).unwrap();

                return ManagedQueryPool {
                    pools: Arc::downgrade(&self.pools),
                    size,
                    pool,
                    next_index: 0,
                    objects: Vec::new(),
                };
            }
        }

        let pool = self
            .device
            .create_query_pool(&QueryPoolDescriptor {
                kind: QueryKind::Timestamp,
                count,
            })
            .unwrap();

        ManagedQueryPool {
            pools: Arc::downgrade(&self.pools),
            size: count,
            pool: Arc::new(pool),
            next_index: 0,
            objects: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ManagedQueryPool {
    pools: Weak<Mutex<BTreeMap<NonZeroU32, Arc<QueryPool>>>>,
    size: NonZeroU32,
    pub pool: Arc<QueryPool>,
    next_index: u32,
    pub objects: Vec<QueryObject>,
}

impl ManagedQueryPool {
    pub fn pool(&self) -> &QueryPool {
        &self.pool
    }

    pub fn next_index(&mut self, object: QueryObject) -> u32 {
        let index = self.next_index;
        assert!(index < self.size.get());

        self.objects.push(object);
        self.next_index += 1;

        index
    }

    pub unsafe fn get(&self) -> Vec<u64> {
        unsafe { self.pool.get(0, self.next_index).unwrap() }
    }
}

impl ManagedQueryPool {
    /// # Safety
    ///
    /// All submissions that write to this pool must have completed.
    pub unsafe fn release(self) {
        if let Some(pools) = self.pools.upgrade() {
            // SAFETY: The caller guarantees that that all submissions
            // that write to this query pool have completed.
            unsafe {
                self.pool.reset();
            }

            pools.lock().insert(self.size, self.pool);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum QueryObject {
    BeginCommands,
    EndCommands,
    BeginPass(&'static str),
    EndPass(&'static str),
}
