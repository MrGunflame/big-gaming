extern crate alloc;

pub mod park;

mod linked_list;
mod loom;
mod task;
mod waker;

use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Poll, RawWaker, RawWakerVTable, Waker};
use std::thread::JoinHandle;

use crossbeam::deque::{Injector, Steal};
use linked_list::LinkedList;
use park::Parker;
use parking_lot::Mutex;
use task::{Header, RawTaskPtr};

pub use task::Task;
use waker::waker_create;

#[derive(Debug)]
pub struct TaskPool {
    inner: Arc<Inner>,
    threads: Option<Vec<JoinHandle<()>>>,
}

#[derive(Debug)]
struct Inner {
    queue: Injector<RawTaskPtr>,
    parker: Parker,
    shutdown: AtomicBool,
    tasks: Mutex<LinkedList<Header>>,
}

impl TaskPool {
    pub fn new(threads: usize) -> Self {
        assert_ne!(threads, 0);

        let inner = Arc::new(Inner {
            queue: Injector::new(),
            parker: Parker::new(),
            shutdown: AtomicBool::new(false),
            tasks: Mutex::new(LinkedList::new()),
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

    pub fn spawn<T, F>(&self, future: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send,
    {
        let task = Task::alloc_new(future, self.inner.clone());
        unsafe {
            self.inner.tasks.lock().push_back(task.header());
        }

        self.inner.queue.push(task);
        self.inner.parker.unpark();

        Task {
            ptr: task,
            _marker: PhantomData,
        }
    }

    pub fn update(&self) {
        // let mut tasks = self.inner.tasks.lock();

        // let mut index = 0;
        // while index < tasks.len() {
        //     let task = tasks[index];
        //     let header = unsafe { &*task.header() };
        //     let state = header.state.load(Ordering::Acquire);

        //     // Task is done, but has no associated `Task` handle.
        //     if state & TASK_REF == 0 && state & (STATE_DONE | STATE_CLOSED) != 0 {
        //         let drop_fn = header.vtable.drop;
        //         unsafe { drop_fn(task.as_ptr()) };
        //         unsafe { task::dealloc_task(task.as_ptr()) };

        //         tasks.remove(index);
        //         continue;
        //     }

        //     index += 1;
        // }
    }

    /// Drops all tasks.
    fn drop_tasks(&mut self) {
        let mut tasks = self.inner.tasks.lock();

        while let Some(ptr) = tasks.head() {
            unsafe {
                tasks.remove(ptr);

                // At this point it is possible that an `Task` handle still exists
                // for this task. We cannot directly delete the task object from
                // here. Instead we decrement the reference count which will cause
                // the object to be deleted once the `Task` handle is dropped.
                let task = RawTaskPtr::from_ptr(ptr.cast().as_ptr());
                task.decrement_ref_count();
            }
        }
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        for _ in 0..self.threads.as_ref().unwrap().len() {
            self.inner.parker.unpark();
        }

        for handle in self.threads.take().unwrap() {
            handle.join().unwrap();
        }

        // All running tasks are now complete.
        self.drop_tasks();
    }
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

fn spawn_worker_thread(inner: Arc<Inner>) -> JoinHandle<()> {
    std::thread::spawn(move || 'out: loop {
        if inner.shutdown.load(Ordering::Acquire) {
            return;
        }

        let task = loop {
            match inner.queue.steal() {
                Steal::Success(task) => break task,
                Steal::Empty => {
                    inner.parker.park();
                    continue 'out;
                }
                Steal::Retry => continue,
            }
        };

        let poll_fn = unsafe { task.as_ptr().cast::<Header>().as_ref().vtable.poll };

        let waker = unsafe { Waker::from_raw(waker_create(task.as_ptr())) };
        match unsafe { poll_fn(task.as_ptr(), &waker as *const Waker) } {
            Poll::Pending => {}
            Poll::Ready(()) => {
                // The `poll` function handles advancing the internal state
                // when the future yields `Ready`.
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
}
