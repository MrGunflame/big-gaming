extern crate alloc;

pub mod park;

mod loom;
mod task;
mod waker;

use std::future::Future;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Poll, RawWaker, RawWakerVTable, Waker};
use std::thread::JoinHandle;

use crossbeam::deque::{Injector, Steal};
use park::Parker;
use task::RawTaskPtr;

pub use task::Task;
use waker::waker_create;

#[derive(Debug)]
pub struct TaskPool {
    inner: Arc<Inner>,
    threads: ManuallyDrop<Vec<JoinHandle<()>>>,
}

#[derive(Debug)]
struct Inner {
    queue: InjectorQueue,
    /// Flag that is set to `true` if the executor no longer polls tasks.
    ///
    /// Once this flag is set no new tasks should be added to the `queue`.
    shutdown: AtomicBool,
}

impl TaskPool {
    /// Creates a new `TaskPool` backed by the given number of threads.
    ///
    /// # Panics
    ///
    /// Panics if `threads` is `0`.
    pub fn new(threads: usize) -> Self {
        assert_ne!(threads, 0);

        let inner = Arc::new(Inner {
            queue: InjectorQueue::new(),
            shutdown: AtomicBool::new(false),
        });

        let mut vec = Vec::new();
        for _ in 0..threads {
            let inner = inner.clone();
            vec.push(spawn_worker_thread(inner));
        }

        Self {
            inner,
            threads: ManuallyDrop::new(vec),
        }
    }

    /// Spawns a new future on the `TaskPool`.
    pub fn spawn<T, F>(&self, future: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        unsafe { self.spawn_unchecked(future) }
    }

    /// Spawns a new future on the `TaskPool` without checking if the future captures local
    /// lifetimes.
    ///
    /// # Safety
    ///
    /// `spawn_unchecked` allows spawning of futures with arbitrary lifetimes. The caller must
    /// guarantee that all lifetimes are valid until the future has finished executing, or was
    /// cancelled.
    pub unsafe fn spawn_unchecked<'a, T, F>(&self, future: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'a,
        T: Send + 'a,
    {
        let task = Task::alloc_new(future, self.inner.clone());
        // unsafe {
        //     self.inner.tasks.lock().push_back(task.header());
        // }

        self.inner.queue.push(task.clone());

        Task {
            ptr: task,
            _marker: PhantomData,
        }
    }

    /// Spawns a future on the `TaskPool` and blocks until the future finishes execution.
    pub fn block_on<T, F>(&self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        futures::executor::block_on(future)
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        // Mark the executor as shutdown.
        // This must happen BEFORE we start draining tasks to prevent
        // queueing of new tasks after we have drained the queue.
        self.inner.shutdown.store(true, Ordering::Release);

        for _ in 0..self.threads.len() {
            self.inner.queue.parker.unpark();
        }

        for handle in unsafe { ManuallyDrop::take(&mut self.threads) } {
            handle.join().unwrap();
        }

        // Drop all task handles that are still in the queue.
        // Since the `shutdown` flag is set no new tasks will be added
        // to the queue.
        while self.inner.queue.pop().is_some() {}
    }
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

fn spawn_worker_thread(inner: Arc<Inner>) -> JoinHandle<()> {
    std::thread::spawn(move || loop {
        if inner.shutdown.load(Ordering::Acquire) {
            return;
        }

        let Some(task) = inner.queue.pop() else {
            inner.queue.parker.park();
            continue;
        };

        let waker = unsafe { Waker::from_raw(waker_create(task.clone())) };
        match unsafe { task.poll(&waker) } {
            Poll::Pending => {}
            Poll::Ready(()) => {
                // The `poll` function handles advancing the internal state
                // when the future yields `Ready`.
            }
        }
    })
}

#[derive(Debug)]
struct InjectorQueue {
    inner: Injector<RawTaskPtr>,
    parker: Parker,
}

impl InjectorQueue {
    fn new() -> Self {
        Self {
            inner: Injector::new(),
            parker: Parker::new(),
        }
    }

    fn push(&self, task: RawTaskPtr) {
        // FIXME: Every unpark still requires a mutex lock which could
        // cause unnecessary delay on high contention.
        self.inner.push(task);
        self.parker.unpark();
    }

    fn pop(&self) -> Option<RawTaskPtr> {
        loop {
            match self.inner.steal() {
                Steal::Success(task) => return Some(task),
                Steal::Empty => return None,
                Steal::Retry => (),
            }
        }
    }
}

fn noop_waker() -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW, |_| {}, |_| {}, |_| {});
    const RAW: RawWaker = RawWaker::new(std::ptr::null(), &VTABLE);
    unsafe { Waker::from_raw(RAW) }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::hint::black_box;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures::future::poll_fn;

    use crate::{noop_waker, TaskPool};

    #[test]
    fn schedule_basic() {
        let executor = TaskPool::new(1);
        let mut task = executor.spawn(async {
            println!("Hello World");
        });

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while Pin::new(&mut task).poll(&mut cx).is_pending() {}
    }

    #[test]
    fn schedule_many() {
        let executor = TaskPool::new(1);
        let mut tasks = Vec::new();
        for _ in 0..1024 {
            let task = executor.spawn(async {
                println!("Hello World");
            });

            tasks.push(task);
        }

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        for mut task in tasks {
            while Pin::new(&mut task).poll(&mut cx).is_pending() {}
        }
    }

    #[test]
    fn schedule_many_threads() {
        let executor = TaskPool::new(8);
        let mut tasks = Vec::new();
        for _ in 0..1024 {
            let task = executor.spawn(async {
                println!("Hello World");
            });

            tasks.push(task);
        }

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        for mut task in tasks {
            while Pin::new(&mut task).poll(&mut cx).is_pending() {}
        }
    }

    #[test]
    fn schedule_yield_pending() {
        let executor = TaskPool::new(1);
        let mut task = executor.spawn(async {
            let mut yielded = false;

            poll_fn(|cx| {
                if yielded {
                    return Poll::Ready(());
                }

                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            })
            .await;
        });

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while Pin::new(&mut task).poll(&mut cx).is_pending() {}
    }

    #[test]
    fn wake_task_when_ready() {
        let executor = TaskPool::new(1);
        let task = executor.spawn(async {
            let mut yielded = false;
            poll_fn(|cx| {
                if yielded {
                    return Poll::Ready(());
                }

                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            })
            .await
        });

        futures::executor::block_on(task);
    }

    #[test]
    fn spawn_then_drop() {
        let executor = TaskPool::new(1);
        let mut tasks = Vec::new();
        for _ in 0..1024 {
            let task = executor.spawn(poll_fn(|_| Poll::<()>::Pending));
            tasks.push(task);
        }

        drop(executor);
        drop(tasks);
    }

    #[test]
    fn read_output() {
        let executor = TaskPool::new(1);
        let task = executor.spawn(async move {
            let val = 1 + 1;
            black_box(val)
        });

        assert_eq!(futures::executor::block_on(task), 2);
    }

    #[test]
    fn task_cancel() {
        let executor = TaskPool::new(1);
        let task = executor.spawn(poll_fn(|cx| {
            cx.waker().wake_by_ref();
            Poll::<()>::Pending
        }));

        let mut future = task.cancel();
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        while Pin::new(&mut future).poll(&mut cx).is_pending() {}
    }

    #[test]
    fn task_future_wake_on_ready() {
        let executor = TaskPool::new(1);
        let mut task = executor.spawn(poll_fn(|cx| {
            cx.waker().wake_by_ref();
            Poll::Ready(())
        }));

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while Pin::new(&mut task).poll(&mut cx).is_pending() {}
    }

    #[test]
    fn task_wake_twice() {
        let executor = TaskPool::new(1);
        let mut task = executor.spawn(poll_fn(|cx| {
            cx.waker().wake_by_ref();
            cx.waker().wake_by_ref();
            Poll::Ready(())
        }));

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while Pin::new(&mut task).poll(&mut cx).is_pending() {}
    }
}
