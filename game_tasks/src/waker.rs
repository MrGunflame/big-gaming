use std::task::{RawWaker, RawWakerVTable};

use crate::task::RawTaskPtr;

const VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub(crate) unsafe fn waker_create(ptr: RawTaskPtr) -> RawWaker {
    unsafe {
        ptr.increment_ref_count();
    }

    RawWaker::new(ptr.as_ptr().as_ptr(), &VTABLE)
}

unsafe fn waker_clone(data: *const ()) -> RawWaker {
    unsafe {
        let task = RawTaskPtr::from_ptr(data);
        task.increment_ref_count();
        RawWaker::new(data, &VTABLE)
    }
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

unsafe fn waker_drop(data: *const ()) {
    unsafe {
        let task = RawTaskPtr::from_ptr(data);
        task.decrement_ref_count();
    }
}
