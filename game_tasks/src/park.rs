//! Thread-parking primitives
//!
//! The [`Parker`] and [`Unparker`] primitives can be used to efficiently put threads to sleep.
//! A single [`Parker`]/[`Unparker`] instance can be used to put multiple threads to sleep.
//!
//! [`Unparker::unpark`] will always wake up a single thread, or if none is available queue the
//! next parking thread to wake up immediately.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

/// A thread parker.
#[derive(Clone, Debug)]
pub struct Parker {
    unparker: Unparker,
}

impl Parker {
    /// Creates a new `Parker`.
    pub fn new() -> Self {
        Self {
            unparker: Unparker {
                inner: Arc::new(Inner {
                    state: AtomicUsize::new(0),
                    mutex: Mutex::new(()),
                    cvar: Condvar::new(),
                }),
            },
        }
    }

    /// Parks the thread until a token becomes available.
    ///
    /// If a token is available `park` will return immediately.
    pub fn park(&self) {
        self.unparker.inner.park();
    }

    /// Returns the [`Unparker`] for this `Parker`.
    pub fn unparker(&self) -> &Unparker {
        &self.unparker
    }
}

/// A thread unparker.
#[derive(Clone, Debug)]
pub struct Unparker {
    inner: Arc<Inner>,
}

impl Unparker {
    /// Unparks a single parked thread.
    ///
    /// If no thread is waiting the next parked thread will wake up immediately.
    pub fn unpark(&self) {
        self.inner.unpark_one();
    }
}

#[derive(Debug)]
struct Inner {
    state: AtomicUsize,
    mutex: Mutex<()>,
    cvar: Condvar,
}

impl Inner {
    fn park(&self) {
        let mut state = self.state.load(Ordering::Acquire);
        while state > 0 {
            match self
                .state
                .compare_exchange(state, state - 1, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return,
                Err(val) => state = val,
            }
        }

        let mut m = self.mutex.lock();

        loop {
            self.cvar.wait(&mut m);

            // Take one token from the pool.
            let mut state = self.state.load(Ordering::Acquire);
            while state > 0 {
                match self.state.compare_exchange(
                    state,
                    state - 1,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => return,
                    Err(val) => state = val,
                }
            }
        }
    }

    fn unpark_one(&self) {
        let state = self.state.fetch_add(1, Ordering::Release);
        assert!(state <= usize::MAX);

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
        let parker = Parker::new();
        let unparker = parker.unparker().clone();

        std::thread::spawn(move || {
            unparker.unpark();
        });

        parker.park();
    }

    #[test]
    fn test_park_many() {
        let parker = Parker::new();
        let unparker = parker.unparker.clone();

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
        let parker = Parker::new();
        let unparker = parker.unparker.clone();
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
