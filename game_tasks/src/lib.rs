extern crate alloc;

pub mod park;

mod task;

use std::future::Future;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Poll, RawWaker, RawWakerVTable, Waker};
use std::thread::JoinHandle;

use crossbeam::deque::{Injector, Steal};
use park::Parker;
use parking_lot::Mutex;
use task::{Header, STATE_CLOSED, STATE_DONE, STATE_QUEUED, STATE_RUNNING, TASK_REF};

pub use task::Task;

#[derive(Debug)]
pub struct TaskPool {
    inner: Arc<Inner>,
    threads: Option<Vec<JoinHandle<()>>>,
}

#[derive(Debug)]
struct Inner {
    queue: Injector<NonNull<()>>,
    parker: Parker,
    shutdown: AtomicBool,
    tasks: Mutex<Vec<NonNull<()>>>,
}

impl TaskPool {
    pub fn new(threads: usize) -> Self {
        assert_ne!(threads, 0);

        let inner = Arc::new(Inner {
            queue: Injector::new(),
            parker: Parker::new(),
            shutdown: AtomicBool::new(false),
            tasks: Mutex::new(vec![]),
        });

        let mut vec = Vec::new();
        for _ in 0..threads {
            let inner = inner.clone();
            vec.push(spawn_worker_thread(inner));
        }

        Self {
            inner,
            threads: Some(vec),
        }
    }

    pub fn spawn<F>(&self, future: F) -> Task<F::Output>
    where
        F: Future<Output = ()> + Send + 'static,
        F::Output: Send,
    {
        let task = Task::alloc_new(future);
        self.inner.tasks.lock().push(task);

        self.inner.queue.push(task);
        self.inner.parker.unparker().unpark_one();

        Task {
            ptr: task,
            _marker: PhantomData,
        }
    }

    pub fn update(&self) {
        let mut tasks = self.inner.tasks.lock();

        let mut index = 0;
        while index < tasks.len() {
            let task = tasks[index];
            let header = unsafe { &*(task.as_ptr() as *const Header) };
            let state = header.state.load(Ordering::Acquire);

            // Task is done, but has no associated `Task` handle.
            if state & TASK_REF == 0 && state & (STATE_DONE | STATE_CLOSED) != 0 {
                let drop_fn = header.vtable.drop;
                unsafe { drop_fn(task) };
                unsafe { task::dealloc_task(task) };

                tasks.remove(index);
                continue;
            }

            index += 1;
        }
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        for _ in 0..self.threads.as_ref().unwrap().len() {
            self.inner.parker.unparker().unpark_one();
        }

        for handle in self.threads.take().unwrap() {
            handle.join().unwrap();
        }

        // All running tasks are now complete.
        self.update();
    }
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

fn spawn_worker_thread(inner: Arc<Inner>) -> JoinHandle<()> {
    std::thread::spawn(move || 'out: loop {
        inner.parker.park();

        if inner.shutdown.load(Ordering::Acquire) {
            return;
        }

        let task = loop {
            match inner.queue.steal() {
                Steal::Success(task) => break task,
                Steal::Empty => continue 'out,
                Steal::Retry => continue,
            }
        };

        let poll_fn = unsafe { task.cast::<Header>().as_ref().vtable.poll };

        let waker = noop_waker();
        loop {
            match unsafe { poll_fn(task, &waker as *const Waker) } {
                Poll::Pending => (),
                Poll::Ready(()) => {
                    let header = task.as_ptr() as *const Header;
                    unsafe {
                        loop {
                            let old_state = (*header).state.load(Ordering::Acquire);
                            let mut new_state = old_state;
                            new_state &= !(STATE_QUEUED | STATE_RUNNING);
                            new_state |= STATE_DONE;

                            if (*header)
                                .state
                                .compare_exchange_weak(
                                    old_state,
                                    new_state,
                                    Ordering::SeqCst,
                                    Ordering::SeqCst,
                                )
                                .is_ok()
                            {
                                break;
                            }
                        }
                    }

                    break;
                }
            }
        }
    })
}

fn noop_waker() -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW, |_| {}, |_| {}, |_| {});
    const RAW: RawWaker = RawWaker::new(std::ptr::null(), &VTABLE);
    unsafe { Waker::from_raw(RAW) }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::Context;

    use crate::{noop_waker, TaskPool};

    #[test]
    fn schedule_basic() {
        let executor = TaskPool::new(1);
        let mut task = executor.spawn(async move {
            println!("Hello World");
        });

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while Pin::new(&mut task).poll(&mut cx).is_pending() {}
    }
}
