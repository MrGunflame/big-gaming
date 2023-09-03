use std::marker::PhantomData;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::sound::Frame;

pub fn channel(size: usize) -> (Sender, Receiver) {
    let buf = Arc::new(Mutex::new(Buffer::new(size)));

    (Sender { inner: buf.clone() }, Receiver { inner: buf })
}

#[derive(Debug)]
pub struct Sender {
    inner: Arc<Mutex<Buffer>>,
}

impl Sender {
    pub fn send(&mut self, frames: &[Frame]) {
        let mut inner = self.inner.lock();
        for &frame in frames {
            inner.push(frame);
        }
    }

    pub fn spare_capacity(&self) -> usize {
        let inner = self.inner.lock();
        inner.spare_cap()
    }
}

#[derive(Debug)]
pub struct Receiver {
    inner: Arc<Mutex<Buffer>>,
}

impl Receiver {
    pub fn recv(&mut self) -> Option<Frame> {
        let mut inner = self.inner.lock();
        inner.pop()
    }
}

#[derive(Debug)]
struct Buffer {
    inner: Vec<Frame>,
    head: usize,
    tail: usize,
}

impl Buffer {
    fn new(size: usize) -> Self {
        Self {
            inner: vec![Frame::EQUILIBRIUM; size],
            head: 0,
            tail: 0,
        }
    }

    fn pop(&mut self) -> Option<Frame> {
        if self.head == self.tail {
            return None;
        }

        let index = self.tail % self.inner.len();
        self.tail += 1;
        Some(self.inner[index])
    }

    fn spare_cap(&self) -> usize {
        self.inner.len() - (self.head - self.tail)
    }

    fn push(&mut self, frame: Frame) {
        assert!(self.spare_cap() > 0);

        let index = self.head % self.inner.len();
        self.head += 1;
        self.inner[index] = frame;
    }
}
