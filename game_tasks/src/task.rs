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

use futures::future::FusedFuture;
use futures::task::AtomicWaker;

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
/// In the initial state the task is `QUEUED` and one handle exists.
const INITIAL_STATE: usize = STATE_QUEUED | REF_COUNT;

#[derive(Debug)]
struct Vtable {
    poll: unsafe fn(NonNull<()>, *const Waker) -> Poll<()>,
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

        match this.header.state.load(Ordering::Acquire) & (STATE_DONE | STATE_CLOSED) {
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

    unsafe fn poll(ptr: NonNull<()>, waker: *const Waker) -> Poll<()> {
        let task = unsafe { ptr.cast::<Self>().as_ref() };

        let header = &task.header;
        let stage = unsafe { &mut *task.stage.get() };
        let task_waker = &task.waker;

        let future = unsafe { &mut stage.future };

        // Unset the `QUEUED` flag and set the `RUNNING` flag.
        // This must happen before we start polling the future.
        let mut state = header.state.load(Ordering::Acquire);

        // Completed/Closed tasks will not get scheduled again,
        // but it is possible for the task to get scheduled while
        // it is still running and then get closed.
        // In this case the task is still scheduled, but we must
        // not poll the future again.
        if state & (STATE_DONE | STATE_CLOSED) != 0 {
            task_waker.wake();
            return Poll::Ready(());
        }

        loop {
            debug_assert_ne!(state & STATE_QUEUED, 0);
            debug_assert_eq!(state & STATE_RUNNING, 0);
            let new_state = state & !STATE_QUEUED | STATE_RUNNING;

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

        let mut cx = Context::from_waker(unsafe { &*waker });
        let pin: Pin<&mut F> = unsafe { Pin::new_unchecked(future) };
        let res = F::poll(pin, &mut cx);

        // Unset the `RUNNING` flag after polling the task.
        // Note that we are reusing the state loaded before
        // polling the task. If the state is outdated the CAS
        // will update it.
        loop {
            let new_state = state & !STATE_RUNNING;
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

        match res {
            Poll::Pending if state & STATE_CANCEL != 0 => {
                unsafe {
                    ManuallyDrop::drop(future);
                }

                loop {
                    let new_state = state | STATE_CLOSED;
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

                task_waker.wake();

                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => {
                unsafe {
                    ManuallyDrop::drop(future);
                    // Write the final value **BEFORE** waking.
                    stage.output = ManuallyDrop::new(val);
                }

                // Set the `DONE` bit (and remove the `QUEUED | RUNNING` bits).
                // This must happen after the output value has been written, but
                // before calling the waker.
                // As soon as the `DONE` bit is set another thread is allowed to
                // read the output value.

                loop {
                    let new_state = state | STATE_DONE;
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

                task_waker.wake();

                Poll::Ready(())
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

    pub(crate) fn waker(&self) -> *const AtomicWaker {
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
            (header.vtable.drop)(self.ptr);
            dealloc_task(self.ptr);
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
    pub(crate) unsafe fn poll(&self, waker: *const Waker) -> Poll<()> {
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
                Ok(_) => break,
                Err(s) => state = s,
            }
        }

        header.executor.queue.push(self.clone());
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
    pub(crate) fn alloc_new<F>(future: F, executor: Arc<Inner>) -> RawTaskPtr
    where
        F: Future<Output = T>,
    {
        let layout = <RawTask<T, F>>::LAYOUT;

        let ptr = unsafe { alloc::alloc::alloc(layout) as *mut RawTask<T, F> };
        if ptr.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }

        let task = RawTask {
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

        unsafe { ptr.write(task) };

        RawTaskPtr {
            ptr: NonNull::new(ptr as *mut ()).unwrap(),
        }
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
        unsafe {
            self.detach_inner();
        }
    }

    /// Detaches the task.
    ///
    /// # Safety
    ///
    /// The task must not be accessed after this call.
    #[inline]
    unsafe fn detach_inner(&self) {
        // SAFETY: We own one of the reference counts and the caller guarantees
        // that this function is only called once.
        // unsafe {
        //     self.ptr.decrement_ref_count();
        // }
    }

    /// Sets this `Task` as being cancelled.
    ///
    /// This should only be called once in the lifetime of the `Task`.
    fn set_cancelled(&self) {
        let header = unsafe { self.ptr.header().as_ref() };

        let mut state = header.state.load(Ordering::Acquire);
        loop {
            debug_assert!(state & STATE_CLOSED == 0);

            // We can't cancel the task if it is already complete.
            if state & (STATE_DONE | STATE_CLOSED) != 0 {
                break;
            }

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
        self.set_cancelled();
        Cancel { task: self }
    }

    /// Cancells the `Task` without waiting for the task be cancelled.
    ///
    /// If the future just completed the value is returned. Otherwise the `Task` is detached and
    /// cancelled in the background.
    ///
    /// This function is a more efficient version of `self.cancel().now_or_never()`.
    pub fn cancel_now(mut self) -> Option<T> {
        self.set_cancelled();
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
            self.detach_inner();
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

unsafe fn dealloc_task(ptr: NonNull<()>) {
    let layout = unsafe { (*(ptr.as_ptr() as *const Header)).vtable.layout };

    unsafe { alloc::alloc::dealloc(ptr.as_ptr() as *mut u8, layout) };
}

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
