//! Thread-parking primitives
//!
//! The [`Parker`] and [`Unparker`] primitives can be used to efficiently put threads to sleep.
//! A single [`Parker`]/[`Unparker`] instance can be used to put multiple threads to sleep.
//!
//! [`Unparker::unpark`] will always wake up a single thread, or if none is available queue the
//! next parking thread to wake up immediately.

use crate::loom::sync::atomic::{AtomicUsize, Ordering};
use crate::loom::sync::{Arc, Condvar, Mutex};

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

    fn unpark_one(&self) {
        // In order for the parked thread to observe the write to `state` we need to
        // perform a `Release` operation that the parked thread can synchronize with.
        let state = self.state.fetch_add(1, Ordering::Release);
        assert!(state < usize::MAX);

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
