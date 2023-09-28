use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

pub const STATE_QUEUED: usize = 1;
pub const STATE_RUNNING: usize = 1 << 1;
pub const STATE_DONE: usize = 1 << 2;
pub const STATE_CLOSED: usize = 1 << 3;

pub struct Vtable {
    pub poll: unsafe fn(NonNull<()>, cx: *const Waker) -> Poll<()>,
    pub drop: unsafe fn(NonNull<()>),
    pub read_output: unsafe fn(NonNull<()>) -> *const (),
}

pub struct Header {
    pub state: AtomicUsize,
    pub vtable: &'static Vtable,
}

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

pub struct Task<T> {
    /// Untyped task pointer.
    pub(crate) ptr: NonNull<()>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Task<T> {
    pub fn alloc_new<F>(future: F) -> NonNull<()>
    where
        F: Future<Output = T>,
    {
        let layout = Layout::new::<RawTask<T, F>>();

        let ptr = unsafe { alloc::alloc::alloc(layout) as *mut RawTask<T, F> };
        assert!(!ptr.is_null());

        let task = RawTask {
            header: Header {
                state: AtomicUsize::new(STATE_QUEUED),
                vtable: &Vtable {
                    poll: RawTask::<T, F>::poll,
                    drop: RawTask::<T, F>::drop,
                    read_output: RawTask::<T, F>::read_output,
                },
            },
            future: ManuallyDrop::new(future),
            output: MaybeUninit::uninit(),
        };

        unsafe { ptr.write(task) };

        NonNull::new(ptr as *mut ()).unwrap()
    }

    fn poll_inner(&mut self, cx: &mut Context<'_>) -> Poll<T> {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;

        unsafe {
            let state = (*header).state.load(Ordering::Acquire);

            match state {
                STATE_QUEUED | STATE_RUNNING => return Poll::Pending,
                STATE_DONE => {
                    loop {
                        // Advance the state from `STATE_DONE` to `STATE_CLOSED`.
                        // Only if the operation succeeds are we allowed to take
                        // the output value.
                        match (*header).state.compare_exchange_weak(
                            STATE_DONE,
                            STATE_CLOSED,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            Ok(_) => break,
                            Err(state) => {
                                // Task was already stolen by another thread.
                                if state != STATE_DONE {
                                    panic!()
                                }
                            }
                        }
                    }

                    let output_ptr = ((*header).vtable.read_output)(self.ptr);
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
}

impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_inner(cx)
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {}
}

struct OwnedTask {
    pub ptr: NonNull<()>,
}

impl OwnedTask {}
