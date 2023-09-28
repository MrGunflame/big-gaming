use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

const EMPTY: usize = 0;
const PARKED: usize = 1;
const NOTIFIED: usize = 2;

#[derive(Clone, Debug)]
pub struct Parker {
    inner: Arc<Inner>,
}

impl Parker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                state: AtomicUsize::new(0),
                mutex: Mutex::new(()),
                cvar: Condvar::new(),
            }),
        }
    }

    pub fn park(&self) {
        self.inner.park();
    }

    pub fn unparker(&self) -> Unparker {
        Unparker {
            inner: self.inner.clone(),
        }
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
        if self
            .state
            .compare_exchange(NOTIFIED, EMPTY, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return;
        }

        let mut m = self.mutex.lock();

        match self
            .state
            .compare_exchange(EMPTY, PARKED, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => (),
            Err(NOTIFIED) => {
                self.state.store(EMPTY, Ordering::SeqCst);
                return;
            }
            Err(_) => unreachable!(),
        }

        loop {
            self.cvar.wait(&mut m);
            if self
                .state
                .compare_exchange(NOTIFIED, EMPTY, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return;
            }
        }
    }

    fn unpark_one(&self) {
        match self.state.swap(NOTIFIED, Ordering::Release) {
            // No one waiting.
            EMPTY => return,
            // Already notified.
            NOTIFIED => return,
            PARKED => (),
            _ => unreachable!(),
        }

        drop(self.mutex.lock());
        self.cvar.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::Parker;

    #[test]
    fn test_park() {
        let parker = Parker::new();
        let unparker = parker.unparker();

        std::thread::spawn(move || {
            unparker.unpark_one();
        });

        parker.park();
    }
}
