use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

#[derive(Clone, Debug)]
pub struct Parker {
    unparker: Unparker,
}

impl Parker {
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

    pub fn park(&self) {
        self.unparker.inner.park();
    }

    pub fn unparker(&self) -> &Unparker {
        &self.unparker
    }
}

#[derive(Clone, Debug)]
pub struct Unparker {
    inner: Arc<Inner>,
}

impl Unparker {
    pub fn unpark_one(&self) {
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
            unparker.unpark_one();
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
                unparker.unpark_one();
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
            unparker.unpark_one();
        }

        barrier.wait();
    }
}
