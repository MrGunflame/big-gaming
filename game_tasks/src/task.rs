use std::alloc::Layout;
use std::cell::UnsafeCell;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use atomic_waker::AtomicWaker;
use futures_core::FusedFuture;

use crate::{noop_waker, Inner};

pub const REF_COUNT: usize = 0b0010_0000;
pub const REF_COUNT_MASK: usize = usize::MAX & !STATE_MASK;

/// Set when the task is currently queued up for execution.
///
/// Note that this flag is **not** mutually exclusive with [`STATE_RUNNING`]. It is possible for a
/// task to be rescheduled while it is being ran.
pub const STATE_QUEUED: usize = 0b0001;

/// Set when the task is currently running.
///
/// Note that this flag is **not** mutually exclusive with [`STATE_QUEUED`]. It is possible for a
/// task to be rescheduled while it is being ran.
pub const STATE_RUNNING: usize = 0b0010;

/// Set when the task has finished and the output value exists.
///
/// This flag is mututally exclusive with [`STATE_CLOSED`].
pub const STATE_DONE: usize = 0b0100;

/// Set in any of the following cases:
/// - The task has finished and the output value has been read.
/// - The task has been cancelled.
///
/// This flag is mutually exclusive with [`STATE_DONE`].
pub const STATE_CLOSED: usize = 0b1000;

/// Set if the task should be cancelled.
pub const STATE_CANCEL: usize = 0b0001_0000;

pub const STATE_MASK: usize =
    STATE_QUEUED | STATE_RUNNING | STATE_DONE | STATE_CLOSED | STATE_CANCEL;

/// The initial state of a [`RawTask`].
///
/// In the initial state the task is `QUEUED` and two handles exists.
const INITIAL_STATE: usize = STATE_QUEUED | (REF_COUNT * 2);

#[derive(Debug)]
struct Vtable {
    poll: unsafe fn(NonNull<()>, *const Waker),
    drop: unsafe fn(NonNull<()>),
    layout: Layout,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Header {
    state: AtomicUsize,
    vtable: &'static Vtable,
    executor: Arc<Inner>,
}

// Casting `RawTask` to `Header` requires the header to be at
// the start of the allocation.
#[repr(C)]
pub(crate) struct RawTask<T, F> {
    header: Header,
    waker: ManuallyDrop<AtomicWaker>,
    stage: UnsafeCell<Stage<T, F>>,
}

impl<T, F> RawTask<T, F>
where
    F: Future<Output = T>,
{
    const LAYOUT: Layout = Layout::new::<Self>();

    unsafe fn drop(ptr: NonNull<()>) {
        // The caller guarantees that this function is only called when
        // a single pointer to this task is left. We can therfore borrow
        // mutably.
        let this = unsafe { ptr.cast::<Self>().as_mut() };

        unsafe {
            ManuallyDrop::drop(&mut this.waker);
        }

        match *this.header.state.get_mut() & (STATE_DONE | STATE_CLOSED) {
            // The `DONE` flag indicates that future has completed and been dropped.
            // The output value has not been consumed.
            // We must drop the output value in this case.
            STATE_DONE => unsafe { ManuallyDrop::drop(&mut this.stage.get_mut().output) },
            // The `CLOSED` flag indicates that the future has been dropped.
            // The output value has either never been written or has already been consumed.
            // We must not drop anything in this case.
            STATE_CLOSED => (),
            // If neither the `DONE` nor `CLOSED` flags are set, the future has not been
            // dropped.
            // We must drop the future in this case.
            _ => unsafe { ManuallyDrop::drop(&mut this.stage.get_mut().future) },
        }

        // Drop the Header last.
        unsafe {
            core::ptr::drop_in_place(ptr.cast::<Header>().as_ptr());
        }
    }

    unsafe fn poll(ptr: NonNull<()>, waker: *const Waker) {
        let task = unsafe { ptr.cast::<Self>().as_ref() };

        let header = &task.header;
        let stage = unsafe { &mut *task.stage.get() };
        let task_waker = &task.waker;

        let future = unsafe { &mut stage.future };

        let mut state = header.state.load(Ordering::Acquire);

        // Unset the `QUEUED` flag and set the `RUNNING` flag.
        // This must happen before we start polling the future.
        loop {
            // To poll a future:
            // - The future must not be polled currently.
            // - The future must not have completed yet.
            // - The future must not have been dropped yet.
            debug_assert_eq!(state & STATE_RUNNING, 0);
            debug_assert_eq!(state & STATE_DONE, 0);
            debug_assert_eq!(state & STATE_CLOSED, 0);

            debug_assert_ne!(state & STATE_QUEUED, 0);

            let new_state = state & !STATE_QUEUED | STATE_RUNNING;

            match header.state.compare_exchange_weak(
                state,
                new_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(s) => {
                    state = s;
                    break;
                }
                Err(s) => state = s,
            }
        }

        let mut cx = Context::from_waker(unsafe { &*waker });
        let pin: Pin<&mut F> = unsafe { Pin::new_unchecked(future) };
        let res = F::poll(pin, &mut cx);

        match res {
            Poll::Pending => {
                loop {
                    let mut new_state = state & !STATE_RUNNING;
                    if state & STATE_CANCEL != 0 {
                        new_state |= STATE_CLOSED;
                    }

                    match header.state.compare_exchange_weak(
                        state,
                        new_state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(s) => {
                            state = s;
                            break;
                        }
                        Err(s) => state = s,
                    }
                }

                // If the future was marked as cancelled we have marked
                // it as `CLOSED` and will drop the future here.
                // The state may also have the `QUEUED` bit set, but it has
                // no effect when the `CLOSED` bit is set, so we don't need
                // to unset it.
                if state & STATE_CLOSED != 0 {
                    unsafe {
                        ManuallyDrop::drop(future);
                    }

                    // Wake the `Task` handle to indicate that the future
                    // has finished cancellation.
                    task_waker.wake();
                }

                // If the future was awoken while we called poll, the waker
                // will set the `QUEUED` bit without scheduling it, so we must
                // schedule it.
                // As soon as we schedule it we must no longer access any exclusive
                // resources of this task as another thread may immediately start
                // polling the task again.
                if state & STATE_QUEUED != 0 && state & STATE_CANCEL == 0 {
                    // Create a new task ptr, pointing at the current `RawTask`.
                    // Note that the caller of `RawTask::poll` still owns the
                    // the `ptr`, so we must increment the ref count for our
                    // new `RawTaskPtr`.
                    let task = RawTaskPtr { ptr };

                    // SAFETY:
                    // The underlying pointer points to a valid `RawTask`.
                    unsafe {
                        task.increment_ref_count();
                    }

                    header.executor.queue.push(task);
                }
            }
            Poll::Ready(val) => {
                unsafe {
                    ManuallyDrop::drop(future);
                    // Write the final value **BEFORE** waking.
                    stage.output = ManuallyDrop::new(val);
                }

                // Set the `DONE` bit to indicate the future was dropped and
                // the output value has been written.
                // As soon as the `DONE` bit is set the output value is allowed
                // to be consumed by another thread.
                // The state may also have the `QUEUED` bit set, but it has
                // no effect when the `DONE` bit is set, so we don't need to
                // unset it.
                loop {
                    let new_state = state & !STATE_RUNNING | STATE_DONE;

                    match header.state.compare_exchange_weak(
                        state,
                        new_state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break,
                        Err(s) => state = s,
                    }
                }

                // Wake the `Task` handle.
                task_waker.wake();
            }
        }
    }
}

/// An opaque pointer to a typed [`RawTask`].
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct RawTaskPtr {
    ptr: NonNull<()>,
}

impl RawTaskPtr {
    pub(crate) fn header(&self) -> NonNull<Header> {
        self.ptr.cast()
    }

    pub(crate) fn as_ptr(&self) -> NonNull<()> {
        self.ptr
    }

    fn waker(&self) -> *const AtomicWaker {
        let offset = size_of::<Header>();
        unsafe { self.ptr.as_ptr().cast::<u8>().add(offset) as *const AtomicWaker }
    }

    pub unsafe fn from_ptr(ptr: *const ()) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr.cast_mut()) },
        }
    }

    /// Increments the ref count.
    ///
    /// # Safety
    ///
    /// - Must not be called on a dangling `RawTaskPtr`.
    pub unsafe fn increment_ref_count(&self) {
        let header = unsafe { self.header().as_ref() };

        let old_rc = header.state.fetch_add(REF_COUNT, Ordering::Relaxed);

        if old_rc > isize::MAX as usize {
            std::process::abort();
        }
    }

    /// Decrements the ref count. If the last ref count is dropped the task deallocated and this
    /// `RawTaskPtr` becomes dangling.
    ///
    /// # Safety
    ///
    /// - This function must not be called on a dangling `RawTaskPtr` value. A `RawTaskPtr` becomes
    ///   dangling when the last ref-count was removed with this function.
    /// - Removing the last ref-count will cause the underlying [`RawTask`] to be dropped on the
    ///   calling thread. As such [`RawTask`] must be safe to drop on this thread (`Send` and `Sync`
    ///   must be guaranteed).
    pub unsafe fn decrement_ref_count(&self) {
        let header = unsafe { self.header().as_ref() };

        // We need to synchronize with the other thread if we are going
        // to deallocate the task.
        // Note that masking for the reference count bits is necessary since
        // state can still contain other flags.
        if header.state.fetch_sub(REF_COUNT, Ordering::Release) & REF_COUNT_MASK != REF_COUNT {
            return;
        }

        // This fence is required to prevent reordering of the use of the task
        // (from another thread) and us deleting the task. Because the previous
        // decrement of the reference count is using `Release` ordering, this
        // `Acquire` will synchronize with with the store, causing any data
        // access to happen before the deletion of the task.
        header.state.load(Ordering::Acquire);

        // We now have exclusive access to the data in the `RawTask` and
        // can safely drop it and deallocate the memory.
        unsafe {
            // Get the allocation parameters before we drop the task,
            // which may invalidate the data in `self.ptr`.
            let ptr = self.ptr.as_ptr() as *mut u8;
            let layout = self.header().as_ref().vtable.layout;

            // Call the `RawTask` drop impl.
            (header.vtable.drop)(self.ptr);

            // Deallocate the `RawTask` memory.
            // Safety: We are calling this with the same values obtained in
            // `RawTask::new`.
            alloc::alloc::dealloc(ptr, layout);
        }
    }

    /// Polls the underlying task for any progress using the given `waker`.
    ///
    /// # Safety
    ///
    /// - The `RawTaskPtr` must not be dangling.
    /// - If the `RawTaskPtr` was moved across threads since creation, `T` and `F` of the
    ///   underlying [`RawTask`] must be `Send`.
    /// - `poll` must not be called from more than one thread at a time.
    #[inline]
    pub(crate) unsafe fn poll(&self, waker: &Waker) {
        unsafe {
            let poll_fn = self.header().as_ref().vtable.poll;
            poll_fn(self.ptr, waker)
        }
    }

    /// Schedules this task for execution.
    ///
    /// # Safety
    ///
    /// - Must not be called on a dangling `RawTaskPtr`.
    pub(crate) unsafe fn schedule(&self) {
        let header = unsafe { self.header().as_ref() };

        // If the shutdown flag is set the executor will no longer
        // poll the future and we do not schedule the task.
        // We must not push the task into the executor queue to
        // prevent a reference cycle.
        if header.executor.shutdown.load(Ordering::Acquire) {
            return;
        }

        let mut state = header.state.load(Ordering::Acquire);
        loop {
            // If a task is done or canceled we must not poll it again.
            // We must not queue the task.
            // If the task is already queued it is already in queue.
            // We must not push it again, otherwise it would be possible
            // for the same task the get polled on multiple threads simultaneously.
            if state & (STATE_QUEUED | STATE_DONE | STATE_CANCEL) != 0 {
                return;
            }

            let new_state = state | STATE_QUEUED;

            match header.state.compare_exchange_weak(
                state,
                new_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(s) => {
                    state = s;
                    break;
                }
                Err(s) => state = s,
            }
        }

        // If the future is currently not being polled we will schedule it.
        // Otherwise we MUST NOT schedule the future while it is
        // being polled currently to prevent another thread from starting
        // to poll it again while the current poll is not complete.
        // If the future is being polled currently we will only set the
        // `QUEUED` bit and it is the responsibility of the poller to
        // schedule the future after it is done polling.
        if state & STATE_RUNNING == 0 {
            header.executor.queue.push(self.clone());
        }
    }

    /// Reads the final output value.
    ///
    /// # Safety
    ///
    /// - This function must only be called once the future has been completed, dropped and the
    ///   output value has been written. This is the case when [`poll`] returns `Poll::Ready(())`.
    /// - The function must not be called more than once on the same `RawTaskPtr`.
    /// - `T` must be the same type as the `T` value of the underlying [`RawTask`].
    /// - `T` must be safe to read from the callers thread, i.e. if `RawTaskPtr` was moved across
    ///   threads since creation, `T` must be `Send`.
    ///
    /// [`poll`]: Self::poll
    #[inline]
    unsafe fn read_output<T>(&self) -> T {
        unsafe {
            // We don't care about the future type but when this function is called
            // the future is already dropped. The dropped future and the output value
            // share the same memory so casting from `RawTask<T, F>` to `RawTask<T, ()>`
            // is possible.
            let task = self.ptr.cast::<RawTask<T, ()>>().as_ref();
            ManuallyDrop::take(&mut (*task.stage.get()).output)
        }
    }
}

impl Clone for RawTaskPtr {
    fn clone(&self) -> Self {
        unsafe {
            self.increment_ref_count();
        }
        Self { ptr: self.ptr }
    }
}

impl Drop for RawTaskPtr {
    fn drop(&mut self) {
        unsafe {
            self.decrement_ref_count();
        }
    }
}

// `RawTaskPtr` itself can shared between threads safely.
// Note that `RawTaskPtr` only gives access to the underlying
// future/output value via `poll` or `drop`, which are unsafe
// since we cannot guarantee that the underlying future/output
// value is also threadsafe.
unsafe impl Sync for RawTaskPtr {}

pub struct Task<T> {
    /// Untyped task pointer.
    pub(crate) ptr: RawTaskPtr,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Task<T> {
    /// Creates a new task and returns a `Task` and a [`RawTaskPtr`] to schedule.
    pub(crate) fn new<F>(future: F, executor: Arc<Inner>) -> (Self, RawTaskPtr)
    where
        F: Future<Output = T>,
    {
        let layout = <RawTask<T, F>>::LAYOUT;

        let ptr = unsafe { alloc::alloc::alloc(layout) as *mut RawTask<T, F> };
        if ptr.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }

        let mut task = RawTask {
            header: Header {
                state: AtomicUsize::new(INITIAL_STATE),
                vtable: &Vtable {
                    poll: RawTask::<T, F>::poll,
                    drop: RawTask::<T, F>::drop,
                    layout: RawTask::<T, F>::LAYOUT,
                },
                executor,
            },
            waker: ManuallyDrop::new(AtomicWaker::new()),
            stage: UnsafeCell::new(Stage {
                future: ManuallyDrop::new(future),
            }),
        };

        // Our initial state contains a refcount of two.
        // - One for the `Task` handle that we return.
        // - One for the `RawTaskPtr` that we return for scheduling.
        debug_assert_eq!(*task.header.state.get_mut() & REF_COUNT_MASK, REF_COUNT * 2);

        unsafe { ptr.write(task) };

        (
            Self {
                ptr: unsafe { RawTaskPtr::from_ptr(ptr.cast_const().cast::<()>()) },
                _marker: PhantomData,
            },
            RawTaskPtr {
                ptr: NonNull::new(ptr.cast::<()>()).unwrap(),
            },
        )
    }

    fn poll_inner(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let header = self.ptr.header();
        let waker = unsafe { &*self.ptr.waker() };
        // The waker might be different that on the last
        // call to `poll`. Since `AtomicWaker` doesn't allow
        // `will_wake`.
        waker.register(cx.waker());

        let mut state = unsafe { header.as_ref().state.load(Ordering::Acquire) };
        let header = unsafe { header.as_ref() };

        match state & (STATE_DONE | STATE_CLOSED) {
            // The future is done and we can read the final output value.
            STATE_DONE => {
                loop {
                    // It is not possible for another thread to "steal" the `DONE` value
                    // since there exists only one `Task` handle.
                    debug_assert!(state & STATE_DONE != 0);
                    debug_assert!(state & STATE_CLOSED == 0);
                    let new_state = state & !STATE_DONE | STATE_CLOSED;

                    match header.state.compare_exchange_weak(
                        state,
                        new_state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break,
                        Err(s) => state = s,
                    }
                }

                let output: T = unsafe { self.ptr.read_output() };
                Poll::Ready(Some(output))
            }
            STATE_CLOSED => Poll::Ready(None),
            _ => Poll::Pending,
        }
    }

    pub fn get_output(&mut self) -> Option<T> {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match self.poll_inner(&mut cx) {
            Poll::Pending => None,
            Poll::Ready(val) => val,
        }
    }

    /// Returns `true` if the `Task` is finished.
    #[inline]
    pub fn is_finished(&self) -> bool {
        let state = unsafe { self.ptr.header().as_ref().state.load(Ordering::Acquire) };
        state & (STATE_DONE | STATE_CLOSED) != 0
    }

    /// Detaches the `Task`, letting it continue in the background.
    #[inline]
    pub fn deatch(self) {
        // Safety: Since we consume self by value this function can not be
        // called again.
        unsafe {
            self.set_detached();
        }
    }

    /// Detaches the task, letting it continue in the background.
    ///
    /// # Safety
    ///
    /// - Must only be called once for the lifetime of this `Task`.
    unsafe fn set_detached(&self) {}

    /// Sets this `Task` as being cancelled.
    ///
    /// # Safety
    ///
    /// - Must only be called once for the lifetime of this `Task`.
    unsafe fn set_cancelled(&self) {
        let header = unsafe { self.ptr.header().as_ref() };

        let mut state = header.state.load(Ordering::Acquire);
        loop {
            debug_assert!(state & STATE_CLOSED == 0);

            // The caller guarantees that this function is only called once,
            // so that only this function has the possibility to set the
            // `CANCEL` bit.
            debug_assert_eq!(state & STATE_CANCEL, 0);

            // We can't cancel the task if it is already complete.
            if state & (STATE_DONE | STATE_CLOSED) != 0 {
                break;
            }

            // If the task is currently not queued or running reschedule it in order
            // for the `Task` waker to be awoken before the future is dropped.
            let new_state = if state & (STATE_QUEUED | STATE_RUNNING) == 0 {
                state | STATE_QUEUED | STATE_CANCEL
            } else {
                state | STATE_CANCEL
            };

            match header.state.compare_exchange_weak(
                state,
                new_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // If the task is currently not queued or running we must schedule
                    // it one final time in order for the `Task` waker to be awoken.
                    if state & (STATE_QUEUED | STATE_RUNNING) == 0 {
                        header.executor.queue.push(self.ptr.clone());
                    }

                    break;
                }
                Err(s) => state = s,
            }
        }
    }

    /// Cancells the `Task` and returns a future that completes once the `Task` has been
    /// cancelled.
    ///
    /// If the returned future is dropped the task is detached and cancelled in the background.
    pub fn cancel(self) -> Cancel<T> {
        // Safety: Since we consume self by value this function can
        // not be called again.
        unsafe {
            self.set_cancelled();
        }

        Cancel { task: self }
    }

    /// Cancells the `Task` without waiting for the task be cancelled.
    ///
    /// If the future just completed the value is returned. Otherwise the `Task` is detached and
    /// cancelled in the background.
    ///
    /// This function is a more efficient version of `self.cancel().now_or_never()`.
    pub fn cancel_now(mut self) -> Option<T> {
        // Safety: Since we consume self by value this function can
        // not be called again.
        unsafe {
            self.set_cancelled();
        }

        let output = self.get_output();
        self.deatch();
        output
    }
}

impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[inline(never)]
        #[cold]
        fn panic_value_consumed() -> ! {
            panic!("`Task` was polled after the future completed");
        }

        self.poll_inner(cx)
            .map(|v| v.unwrap_or_else(|| panic_value_consumed()))
    }
}

impl<T> FusedFuture for Task<T> {
    #[inline]
    fn is_terminated(&self) -> bool {
        let header = self.ptr.header();
        let state = unsafe { header.as_ref().state.load(Ordering::Acquire) };
        state & STATE_CLOSED != 0
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        unsafe {
            self.set_detached();
        }
    }
}

impl<T> Debug for Task<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("ptr", &self.ptr)
            .finish_non_exhaustive()
    }
}

// Because `Task<T>` allows extraction `T` from another thread,
// `T` must be `Send`.
unsafe impl<T> Send for Task<T> where T: Send {}

// Because `&Task<T>` never has any access to the `T` value,
// `Task<T>` is always `Sync`, regardless of `T`.
unsafe impl<T> Sync for Task<T> {}

/// A union containing either the future or the output value of a [`RawTask`].
union Stage<T, F> {
    /// The output value of the future.
    output: ManuallyDrop<T>,
    /// The non-terminated future.
    future: ManuallyDrop<F>,
}

#[derive(Debug)]
pub struct Cancel<T> {
    // Detached on drop.
    task: Task<T>,
}

impl<T> Future for Cancel<T> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.task.poll_inner(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::task::{Poll, Waker};

    use futures_lite::future::poll_fn;

    use crate::Inner;

    use super::Task;

    /// Returns an executor that can be used to allocated [`Task`]s with.
    fn with_executor<F>(f: F)
    where
        F: FnOnce(Arc<Inner>),
    {
        let executor = Arc::new(Inner::new());
        f(executor.clone());

        // Drain the queue at the end dropping all tasks.
        // This is important because the tasks have a reference the executor
        // i.e. a reference cycle exists that we must break to prevent miri
        // from complaining about memory leaks.
        while let Some(_) = executor.queue.pop() {}
    }

    #[test]
    fn task_done_drop_output() {
        with_executor(|executor| {
            let (task, ptr) = Task::new(poll_fn(|_| Poll::Ready(())), executor);

            // The future is ready and `STATE_DONE` is set but the value is
            // never consumed.
            unsafe {
                ptr.poll(Waker::noop());
            }

            drop(task);
        });
    }

    #[test]
    fn task_get_output() {
        with_executor(|executor| {
            let (mut task, ptr) = Task::new(poll_fn(|_| Poll::Ready(())), executor);

            task.get_output();

            assert!(!task.is_finished());
            assert_eq!(task.get_output(), None);

            unsafe {
                ptr.poll(Waker::noop());
            }

            assert!(task.is_finished());
            assert_eq!(task.get_output(), Some(()));
        });
    }

    #[test]
    fn task_detach() {
        with_executor(|executor| {
            let (task, ptr) = Task::new(async move {}, executor);
            drop(ptr);

            task.deatch();
        });
    }

    #[test]
    fn task_cancel_now_pending() {
        with_executor(|executor| {
            let (task, ptr) = Task::new(poll_fn(|_| Poll::<()>::Pending), executor);

            unsafe {
                ptr.poll(Waker::noop());
            }

            assert_eq!(task.cancel_now(), None);
        });
    }

    #[test]
    fn task_cancel_now_ready() {
        with_executor(|executor| {
            let (task, ptr) = Task::new(poll_fn(|_| Poll::Ready(())), executor);

            unsafe {
                ptr.poll(Waker::noop());
            }

            assert_eq!(task.cancel_now(), Some(()));
        });
    }
}
