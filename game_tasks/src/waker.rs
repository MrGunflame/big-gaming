use std::ptr::NonNull;
use std::task::{RawWaker, RawWakerVTable};

use crate::task::RawTaskPtr;

const VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub(crate) fn waker_create(ptr: NonNull<()>) -> RawWaker {
    RawWaker::new(ptr.as_ptr(), &VTABLE)
}

unsafe fn waker_clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

unsafe fn waker_wake(data: *const ()) {
    unsafe {
        waker_wake_by_ref(data);
        waker_drop(data);
    }
}

unsafe fn waker_wake_by_ref(data: *const ()) {
    unsafe {
        let task = RawTaskPtr::from_ptr(data);
        task.schedule();
    }
}

unsafe fn waker_drop(_data: *const ()) {}
