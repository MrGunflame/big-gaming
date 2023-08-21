use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Add, AddAssign, Mul, MulAssign};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::effects::Volume;
use crate::sound_data::SoundData;
use crate::track::TrackId;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct Frame {
    pub left: f32,
    pub right: f32,
}

impl Frame {
    pub const EQUILIBRIUM: Self = Self {
        left: 0.0,
        right: 0.0,
    };
}

impl Add for Frame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left + rhs.left,
            right: self.right + rhs.right,
        }
    }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Mul<Volume> for Frame {
    type Output = Self;

    fn mul(self, rhs: Volume) -> Self::Output {
        Self {
            left: self.left * rhs.0,
            right: self.right * rhs.0,
        }
    }
}

impl MulAssign<Volume> for Frame {
    fn mul_assign(&mut self, rhs: Volume) {
        *self = *self * rhs;
    }
}

#[derive(Debug)]
pub(crate) struct Sender {
    // Note that we do not provide a `Clone` impl, so we
    // can safely assume that `Queue::push` may only be
    // called from one thread.
    //inner: Arc<Queue>,
    inner: Arc<Mutex<VecDeque<Frame>>>,
}

impl Sender {
    pub fn push(&self, value: Frame) {
        //unsafe {
        //    self.inner.push(value);
        //}
        let mut inner = self.inner.lock().unwrap();
        inner.push_back(value);
    }
}

unsafe impl Send for Sender {}

#[derive(Debug)]
pub(crate) struct Receiver {
    //inner: Arc<Queue>,
    inner: Arc<Mutex<VecDeque<Frame>>>,
}

impl Receiver {
    pub fn pop(&self) -> Option<Frame> {
        //unsafe { self.inner.pop() }
        let mut inner = self.inner.lock().unwrap();
        inner.pop_front()
    }
}

unsafe impl Send for Receiver {}

#[derive(Debug)]
pub(crate) struct Queue {
    inner: Box<[UnsafeCell<MaybeUninit<Frame>>]>,
    /// The position of the next write, or in other words, the first **NON**-initialized element.
    head: AtomicUsize,
    /// The position of the next read.
    tail: AtomicUsize,
    // Ensure that we are `!Sync`.
    _marker: PhantomData<*const ()>,
}

impl Queue {
    pub fn new(size: usize) -> Self {
        let mut inner = Vec::with_capacity(size);
        for _ in 0..size {
            inner.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            inner: inner.into_boxed_slice(),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            _marker: PhantomData,
        }
    }

    pub fn split(self) -> (Sender, Receiver) {
        let inner: Arc<Mutex<VecDeque<Frame>>> = Arc::default();
        (
            Sender {
                inner: inner.clone(),
            },
            Receiver { inner },
        )
    }

    pub unsafe fn push(&self, value: Frame) {
        let index = self.head.fetch_add(1, Ordering::Relaxed) % self.inner.len();

        let slot = unsafe { &mut *self.inner[index].get() };
        slot.write(value);
    }

    pub unsafe fn pop(&self) -> Option<Frame> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Relaxed);

        // Caught up to write head; there is no data ready.
        if head == tail {
            return None;
        }

        self.tail.fetch_add(1, Ordering::Relaxed);
        let index = tail % self.inner.len();

        let slot = unsafe { &mut *self.inner[index].get() };
        let val = unsafe { slot.assume_init_read() };
        Some(val)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(pub(crate) slotmap::DefaultKey);

#[derive(Debug)]
pub(crate) struct PlayingSound {
    pub data: SoundData,
    pub cursor: usize,
    pub track: TrackId,
}
