use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::task::{RawWaker, RawWakerVTable};

use crate::task::RawTaskPtr;

const VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub(crate) unsafe fn waker_create(ptr: RawTaskPtr) -> RawWaker {
    let ptr = ManuallyDrop::new(ptr);
    RawWaker::new(ptr.as_ptr().as_ptr(), &VTABLE)
}

unsafe fn waker_clone(data: *const ()) -> RawWaker {
    unsafe {
        let task = ManuallyDrop::new(RawTaskPtr::from_ptr(data));
        waker_create(task.deref().clone())
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
        let task = ManuallyDrop::new(RawTaskPtr::from_ptr(data));
        task.schedule();
    }
}

unsafe fn waker_drop(data: *const ()) {
    unsafe {
        let task = RawTaskPtr::from_ptr(data);
        drop(task);
    }
}

#[cfg(test)]
mod tests {
    use std::task::Waker;

    use crate::{Task, TaskPool};

    use super::waker_create;

    fn new_waker() -> Waker {
        let executor = TaskPool::new(1);
        let task = Task::alloc_new(async {}, executor.inner.clone());
        unsafe { Waker::from_raw(waker_create(task)) }
    }

    #[test]
    fn waker_drop() {
        let waker = new_waker();
        drop(waker);
    }

    #[test]
    fn waker_clone() {
        let waker = new_waker();
        let waker2 = waker.clone();
        drop(waker);
        drop(waker2);
    }

    #[test]
    fn waker_wake() {
        let waker = new_waker();
        waker.wake();
    }

    #[test]
    fn waker_wake_by_ref_twice() {
        let waker = new_waker();
        waker.wake_by_ref();
        waker.wake_by_ref();
    }
}
