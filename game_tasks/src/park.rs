//! Thread-parking primitives
//!
//! The [`Parker`] primitive can be used to efficiently put threads to sleep. A single [`Parker`]
//! instance can be used for multiple threads.

use crate::loom::sync::atomic::{AtomicUsize, Ordering};
use crate::loom::sync::{Condvar, Mutex};

/// A thread parking/unparking token.
#[derive(Debug)]
pub struct Parker {
    state: AtomicUsize,
    mutex: Mutex<()>,
    cvar: Condvar,
}

impl Parker {
    /// Creates a new `Parker`.
    #[cfg(not(loom))]
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
            mutex: Mutex::new(()),
            cvar: Condvar::new(),
        }
    }

    #[cfg(loom)]
    pub fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
            mutex: Mutex::new(()),
            cvar: Condvar::new(),
        }
    }

    /// Puts the calling thread to sleep until it is unparked.
    ///
    /// If a token is available `park` will return immediately. `park` will **not** spuriously
    /// return before it is unparked.
    pub fn park(&self) {
        // To ensure any writes from the unpark operations are be observed we need to
        // perform a `Acquire` load the the unpark thread can synchronize with.
        let mut state = self.state.load(Ordering::Acquire);
        while state > 0 {
            match self.state.compare_exchange_weak(
                state,
                state - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(val) => state = val,
            }
        }

        let mut m = self.mutex.lock().unwrap();

        // It is possible for a token to after checking `state` but before we are going
        // to sleep. The unpark operation will wait until it can acquire the mutex, signaling
        // that we have gone to sleep.
        // If the unpark thread wins the race for the mutex, a token is now available and we
        // must consume it while we have locked the mutex.
        let mut state = self.state.load(Ordering::Acquire);
        while state > 0 {
            match self.state.compare_exchange_weak(
                state,
                state - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(val) => state = val,
            }
        }

        loop {
            m = self.cvar.wait(m).unwrap();

            // Take one token from the pool.
            let mut state = self.state.load(Ordering::Acquire);
            while state > 0 {
                match self.state.compare_exchange_weak(
                    state,
                    state - 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return,
                    Err(val) => state = val,
                }
            }
        }
    }

    /// Unparks a single parked thread.
    ///
    /// If no thread is parked a wakeup token is stored and the next parking thread will
    /// wake up immediately.
    ///
    /// Note that storing more than [`usize::MAX`] tokens results in unspecified behavior.
    pub fn unpark(&self) {
        // In order for the parked thread to observe the write to `state` we need to
        // perform a `Release` operation that the parked thread can synchronize with.
        let state = self.state.fetch_add(1, Ordering::Release);
        debug_assert!(state < usize::MAX);

        // There is a period between the parking thread checking `state` and going
        // to sleep. If we were to notify it during that time, it would go to sleep
        // and never wake up again.
        // During that time the parking thread acquires the mutex and only releases
        // it again after going to sleep. By acquiring the mutex before notifying the
        // parking thread we can guarantee that that it actually went to sleep.
        drop(self.mutex.lock());
        self.cvar.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use super::Parker;

    const NUM_THREADS: usize = 128;

    #[test]
    fn test_park() {
        let parker = Arc::new(Parker::new());
        let unparker = parker.clone();

        std::thread::spawn(move || {
            unparker.unpark();
        });

        parker.park();
    }

    #[test]
    fn test_park_many() {
        let parker = Arc::new(Parker::new());
        let unparker = parker.clone();

        for _ in 0..NUM_THREADS {
            let unparker = unparker.clone();
            std::thread::spawn(move || {
                unparker.unpark();
            });
        }

        for _ in 0..NUM_THREADS {
            parker.park();
        }
    }

    #[test]
    fn park_threads_at_once() {
        let parker = Arc::new(Parker::new());
        let unparker = parker.clone();
        let barrier = Arc::new(Barrier::new(NUM_THREADS + 1));

        for _ in 0..NUM_THREADS {
            let parker = parker.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                parker.park();
                barrier.wait();
            });
        }

        for _ in 0..NUM_THREADS {
            unparker.unpark();
        }

        barrier.wait();
    }
}
