use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

/// A monotonically increasing counter.
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Counter(AtomicU64);

impl Counter {
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    #[inline]
    pub fn set(&self, val: u64) {
        self.0.store(val, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn add(&self, val: u64) {
        self.0.fetch_add(val, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc(&self) {
        self.add(1);
    }
}

impl Clone for Counter {
    #[inline]
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.get()))
    }
}

impl Display for Counter {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.get(), f)
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Gauge(AtomicU64);

impl Gauge {
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    #[inline]
    pub fn set(&self, val: u64) {
        self.0.store(val, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn add(&self, val: u64) {
        self.0.fetch_add(val, Ordering::Relaxed);
    }

    #[inline]
    pub fn sub(&self, val: u64) {
        self.0.fetch_sub(val, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc(&self) {
        self.add(1);
    }

    #[inline]
    pub fn dec(&self) {
        self.sub(1);
    }
}

impl Clone for Gauge {
    #[inline]
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.get()))
    }
}

impl Display for Gauge {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.get(), f)
    }
}
