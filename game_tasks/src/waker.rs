use std::ptr::NonNull;
use std::task::{RawWaker, RawWakerVTable};

use crate::task::{Header, RawTaskPtr};

const VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub(crate) fn waker_create(ptr: NonNull<()>) -> RawWaker {
    RawWaker::new(ptr.as_ptr(), &VTABLE)
}

unsafe fn waker_clone(waker: *const ()) -> RawWaker {
    RawWaker::new(waker, &VTABLE)
}

unsafe fn waker_wake(waker: *const ()) {
    unsafe {
        waker_wake_by_ref(waker);
        waker_drop(waker);
    }
}

unsafe fn waker_wake_by_ref(waker: *const ()) {
    let header = unsafe { &*waker.cast::<Header>() };

    header
        .executor
        .queue
        .push(unsafe { RawTaskPtr::from_ptr(waker) });
    header.executor.parker.unpark();
}

unsafe fn waker_drop(waker: *const ()) {}
