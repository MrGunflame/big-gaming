#[cfg(not(loom))]
pub mod sync {
    pub mod atomic {
        pub use core::sync::atomic::{AtomicUsize, Ordering};
    }

    pub use alloc::sync::Arc;
    pub use std::sync::{Condvar, Mutex};
}

#[cfg(loom)]
pub mod sync {
    pub mod atomic {
        pub use loom::sync::atomic::{AtomicUsize, Ordering};
    }

    pub use loom::sync::{Arc, Condvar, Mutex};
}
