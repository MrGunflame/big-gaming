use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::noop_waker;

pub const STATE_QUEUED: usize = 1;
pub const STATE_RUNNING: usize = 1 << 1;
pub const STATE_DONE: usize = 1 << 2;
pub const STATE_CLOSED: usize = 1 << 3;

/// [`Task`] reference to this task exists.
pub const TASK_REF: usize = 1 << 4;

pub struct Vtable {
    pub poll: unsafe fn(NonNull<()>, cx: *const Waker) -> Poll<()>,
    pub drop: unsafe fn(NonNull<()>),
    pub read_output: unsafe fn(NonNull<()>) -> *const (),
}

pub struct Header {
    pub state: AtomicUsize,
    pub layout: Layout,
    pub vtable: &'static Vtable,
}

// Casting `RawTask` to `Header` requires the header to be at
// the start of the allocation.
#[repr(C)]
pub struct RawTask<T, F>
where
    F: Future<Output = T>,
{
    header: Header,
    future: ManuallyDrop<F>,
    output: MaybeUninit<T>,
}

impl<T, F> RawTask<T, F>
where
    F: Future<Output = T>,
{
    unsafe fn read_output(ptr: NonNull<()>) -> *const () {
        let this = ptr.cast::<Self>().as_ref();
        this.output.as_ptr() as *const ()
    }

    unsafe fn drop(ptr: NonNull<()>) {
        // Header doesn't need Drop.
        assert!(!std::mem::needs_drop::<Header>());

        let this = ptr.cast::<Self>().as_mut();
        ManuallyDrop::drop(&mut this.future);

        if *this.header.state.get_mut() == STATE_DONE {
            this.output.assume_init_drop();
        }
    }

    unsafe fn poll(ptr: NonNull<()>, waker: *const Waker) -> Poll<()> {
        let this = ptr.cast::<Self>().as_mut();

        let mut cx = Context::from_waker(&*waker);
        let pin: Pin<&mut F> = Pin::new_unchecked(&mut this.future);
        match F::poll(pin, &mut cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => {
                this.output.write(val);
                Poll::Ready(())
            }
        }
    }
}

/// An opaque pointer to a typed [`RawTask`].
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub(crate) struct RawTaskPtr {
    ptr: NonNull<()>,
}

impl RawTaskPtr {
    pub(crate) fn header(self) -> *const Header {
        self.ptr.as_ptr() as *const Header
    }

    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.ptr
    }
}

pub struct Task<T> {
    /// Untyped task pointer.
    pub(crate) ptr: RawTaskPtr,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Task<T> {
    pub(crate) fn alloc_new<F>(future: F) -> RawTaskPtr
    where
        F: Future<Output = T>,
    {
        let layout = Layout::new::<RawTask<T, F>>();

        let ptr = unsafe { alloc::alloc::alloc(layout) as *mut RawTask<T, F> };
        assert!(!ptr.is_null());

        let task = RawTask {
            header: Header {
                state: AtomicUsize::new(STATE_QUEUED | TASK_REF),
                vtable: &Vtable {
                    poll: RawTask::<T, F>::poll,
                    drop: RawTask::<T, F>::drop,
                    read_output: RawTask::<T, F>::read_output,
                },
                layout,
            },
            future: ManuallyDrop::new(future),
            output: MaybeUninit::uninit(),
        };

        unsafe { ptr.write(task) };

        RawTaskPtr {
            ptr: NonNull::new(ptr as *mut ()).unwrap(),
        }
    }

    fn poll_inner(&mut self, cx: &mut Context<'_>) -> Poll<T> {
        let header = self.ptr.header();

        unsafe {
            let state = (*header).state.load(Ordering::Acquire);

            match state & (STATE_QUEUED | STATE_RUNNING | STATE_DONE | STATE_CLOSED) {
                STATE_QUEUED | STATE_RUNNING => return Poll::Pending,
                STATE_DONE => {
                    loop {
                        let old_state = (*header).state.load(Ordering::Acquire);
                        let mut new_state = old_state;
                        new_state &= !STATE_DONE;
                        new_state |= STATE_CLOSED;

                        // Advance the state from `STATE_DONE` to `STATE_CLOSED`.
                        // Only if the operation succeeds are we allowed to take
                        // the output value.
                        match (*header).state.compare_exchange_weak(
                            old_state,
                            new_state,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            Ok(_) => break,
                            Err(state) => {
                                // Task was already stolen by another thread.
                                if (state & STATE_DONE) == 0 {
                                    panic!()
                                }
                            }
                        }
                    }

                    let output_ptr = ((*header).vtable.read_output)(self.ptr.as_ptr());
                    let output = std::ptr::read(output_ptr as *const T);
                    return Poll::Ready(output);
                }
                STATE_CLOSED => {
                    // Value already taken.
                    panic!()
                }
                _ => unreachable!(),
            }
        }
    }

    fn detach(&self) {
        let header = unsafe { &*self.ptr.header() };
        let state = header.state.load(Ordering::Acquire);

        // Remove the `TASK_REF` flag.
        debug_assert!(state & TASK_REF != 0);
        while header
            .state
            .compare_exchange_weak(state, state & !TASK_REF, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {}
    }

    pub fn get_output(&mut self) -> Option<T> {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match self.poll_inner(&mut cx) {
            Poll::Pending => None,
            Poll::Ready(val) => Some(val),
        }
    }
}

impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_inner(cx)
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.detach();
    }
}

pub(crate) unsafe fn dealloc_task(ptr: NonNull<()>) {
    let layout = unsafe { (*(ptr.as_ptr() as *const Header)).layout };

    unsafe { alloc::alloc::dealloc(ptr.as_ptr() as *mut u8, layout) };
}
