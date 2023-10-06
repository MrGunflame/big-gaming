use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::task::RawTaskPtr;
use crate::Inner;

const VTABLE: &'static RawWakerVTable = &RawWakerVTable::new(
    WakerData::clone,
    WakerData::wake,
    WakerData::wake_by_ref,
    WakerData::drop,
);

#[derive(Clone, Debug)]
pub(crate) struct WakerData {
    task: RawTaskPtr,
    inner: Arc<Inner>,
}

impl WakerData {
    pub fn new(task: RawTaskPtr, inner: Arc<Inner>) -> Waker {
        unsafe { Waker::from_raw(Self::from_raw(Self { task, inner })) }
    }

    unsafe fn clone(waker: *const ()) -> RawWaker {
        let data = unsafe { &*(waker as *const WakerData) };
        Self::from_raw(data.clone())
    }

    unsafe fn wake_by_ref(waker: *const ()) {
        let data = unsafe { &*(waker as *const WakerData) };
        data.inner.queue.push(data.task);
        data.inner.parker.unpark();
    }

    unsafe fn wake(waker: *const ()) {
        unsafe {
            Self::wake_by_ref(waker);
            Self::drop(waker);
        }
    }

    unsafe fn drop(waker: *const ()) {
        let data = unsafe { Box::from_raw(waker as *mut WakerData) };
        drop(data);
    }

    #[inline]
    fn from_raw(waker: WakerData) -> RawWaker {
        // TODO: It's probably better if we use a fat pointer
        // and avoid the allocation.
        let data = Box::into_raw(Box::new(waker)) as *const ();
        RawWaker::new(data, VTABLE)
    }
}
