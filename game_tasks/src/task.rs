use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::pin::Pin;
use std::ptr::{addr_of_mut, NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use futures::future::FusedFuture;
use futures::task::AtomicWaker;

use crate::linked_list::{Link, Pointers};
use crate::{noop_waker, Inner};

// The first two bits are used for reference counting. We need at most
// 2 references to the task.
pub const REF_COUNT: usize = 1;
pub const REF_COUNT_MASK: usize = (1 << 2) - 1;

pub const STATE_QUEUED: usize = 1 << 2;
pub const STATE_RUNNING: usize = 1 << 3;
pub const STATE_DONE: usize = 1 << 4;
pub const STATE_CLOSED: usize = 1 << 5;

/// The initial state of a [`RawTask`].
///
/// In the initial state the task is `QUEUED` and two handles to it exist (one from the executor)
/// and one from the task handle.
const INITIAL_STATE: usize = STATE_QUEUED | REF_COUNT * 2;

#[derive(Debug)]
pub struct Vtable {
    pub poll: unsafe fn(NonNull<()>, cx: *const Waker) -> Poll<()>,
    pub drop: unsafe fn(NonNull<()>),
    pub read_output: unsafe fn(NonNull<()>) -> *const (),
}

#[derive(Debug)]
#[repr(C)]
pub struct Header {
    pub pointers: Pointers<Header>,
    pub state: AtomicUsize,
    pub layout: Layout,
    pub vtable: &'static Vtable,
    pub executor: Arc<Inner>,
}

unsafe impl Link for Header {
    #[inline]
    unsafe fn pointers(ptr: NonNull<Self>) -> NonNull<Pointers<Self>> {
        ptr.cast()
    }
}

// Casting `RawTask` to `Header` requires the header to be at
// the start of the allocation.
#[repr(C)]
pub struct RawTask<T, F>
where
    F: Future<Output = T>,
{
    header: Header,
    waker: ManuallyDrop<AtomicWaker>,
    future: ManuallyDrop<F>,
    output: MaybeUninit<T>,
}

impl<T, F> RawTask<T, F>
where
    F: Future<Output = T>,
{
    unsafe fn read_output(ptr: NonNull<()>) -> *const () {
        let this = unsafe { ptr.cast::<Self>().as_ref() };
        this.output.as_ptr() as *const ()
    }

    unsafe fn drop(ptr: NonNull<()>) {
        let this = unsafe { ptr.cast::<Self>().as_mut() };
        unsafe {
            ManuallyDrop::drop(&mut this.waker);
            ManuallyDrop::drop(&mut this.future);
        }

        if *this.header.state.get_mut() == STATE_DONE {
            unsafe {
                this.output.assume_init_drop();
            }
        }

        // Drop the Header last.
        unsafe {
            core::ptr::drop_in_place(ptr.cast::<Header>().as_ptr());
        }
    }

    unsafe fn poll(ptr: NonNull<()>, waker: *const Waker) -> Poll<()> {
        let ptr = ptr.cast::<Self>();

        let header = unsafe { &*ptr.cast::<Header>().as_ptr() };
        let task_waker = unsafe { &*addr_of_mut!((*ptr.as_ptr()).waker) };
        let future = unsafe { &mut *addr_of_mut!((*ptr.as_ptr()).future) };
        let output = unsafe { &mut *addr_of_mut!((*ptr.as_ptr()).output) };

        let mut cx = Context::from_waker(unsafe { &*waker });
        let pin: Pin<&mut F> = unsafe { Pin::new_unchecked(future) };
        match F::poll(pin, &mut cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => {
                // Write the final value **BEFORE** waking.
                output.write(val);

                // Set the `DONE` bit (and remove the `QUEUED | RUNNING` bits).
                // This must happen after the output value has been written, but
                // before calling the waker.
                // As soon as the `DONE` bit is set another thread is allowed to
                // read the output value.
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

                task_waker.wake();

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
    pub(crate) fn header(self) -> NonNull<Header> {
        self.ptr.cast()
    }

    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.ptr
    }

    pub(crate) fn waker(self) -> *const AtomicWaker {
        let offset = std::mem::size_of::<Header>();
        unsafe { self.ptr.as_ptr().cast::<u8>().add(offset) as *const AtomicWaker }
    }

    pub unsafe fn from_ptr(ptr: *const ()) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr.cast_mut()) },
        }
    }

    /// Decrements the ref count. If the last ref count is dropped the task deallocated and this
    /// `RawTaskPtr` becomes dangling.
    pub unsafe fn decrement_ref_count(self) {
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
}

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
        let layout = Layout::new::<RawTask<T, F>>();

        let ptr = unsafe { alloc::alloc::alloc(layout) as *mut RawTask<T, F> };
        assert!(!ptr.is_null());

        let task = RawTask {
            header: Header {
                state: AtomicUsize::new(INITIAL_STATE),
                vtable: &Vtable {
                    poll: RawTask::<T, F>::poll,
                    drop: RawTask::<T, F>::drop,
                    read_output: RawTask::<T, F>::read_output,
                },
                layout,
                pointers: Pointers::new(),
                executor,
            },
            waker: ManuallyDrop::new(AtomicWaker::new()),
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
        let waker = unsafe { &*self.ptr.waker() };
        // The waker might be different that on the last
        // call to `poll`. Since `AtomicWaker` doesn't allow
        // `will_wake`.
        waker.register(cx.waker());

        unsafe {
            let state = header.as_ref().state.load(Ordering::Acquire);

            match state & (STATE_QUEUED | STATE_RUNNING | STATE_DONE | STATE_CLOSED) {
                STATE_QUEUED | STATE_RUNNING => return Poll::Pending,
                STATE_DONE => {
                    loop {
                        let old_state = header.as_ref().state.load(Ordering::Acquire);
                        let mut new_state = old_state;
                        new_state &= !STATE_DONE;
                        new_state |= STATE_CLOSED;

                        // Advance the state from `STATE_DONE` to `STATE_CLOSED`.
                        // Only if the operation succeeds are we allowed to take
                        // the output value.
                        match header.as_ref().state.compare_exchange_weak(
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

                    let output_ptr = (header.as_ref().vtable.read_output)(self.ptr.as_ptr());
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
        unsafe {
            self.ptr.decrement_ref_count();
        }
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

impl<T> FusedFuture for Task<T> {
    fn is_terminated(&self) -> bool {
        let header = self.ptr.header();
        let state = unsafe { header.as_ref().state.load(Ordering::Acquire) };
        state & STATE_CLOSED != 0
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
