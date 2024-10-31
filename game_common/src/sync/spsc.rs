use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A lock-free bounded size single-producer, single-consumer queue.
#[derive(Debug)]
pub struct Queue<T> {
    /// The buffer index of the next element to be written to.
    head: AtomicUsize,
    /// The buffer index of the next element to be read from.
    tail: AtomicUsize,
    // Note that for a buffer size of `N` we only store at most `N-1` to avoid
    // the ambiguity of `head == tail`. `head == tail` in the current implementation
    // indicates that the buffer is empty, i.e. no elements are allowed to be read and
    // `head` should insert the next element at index `head == tail`.
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    // Type is not `Sync`.
    _marker: PhantomData<*const ()>,
}

impl<T> Queue<T> {
    /// Creates a new `Queue` with a capacity of `size`.
    pub fn new(size: usize) -> Self {
        let mut buf = Vec::with_capacity(size);
        for _ in 0..size {
            buf.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            buf: buf.into_boxed_slice(),
            _marker: PhantomData,
        }
    }

    /// Pushes a new elements into the queue.
    ///
    /// Returns `Err` with the given `value` if the push operation failed because the `Queue` is
    /// full.
    pub fn push(&self, value: T) -> Result<(), T> {
        // The write `head` is only modified by the thread calling `push`, therefore
        // we always have the correct value in memory, regardless of memory ordering.
        // Since `Queue` is `!Sync` it is not possible to call `push` from more than
        // one thread.
        let head = self.head.load(Ordering::Relaxed);
        let new_head = (head + 1) % self.buffer_capacity();

        let tail = self.tail.load(Ordering::Acquire);

        // We are trying to overwrite the tail element of the ringbuffer.
        // This means we have reached maximum capacity and need to wait
        // for the tail to be moved forward.
        if new_head == tail {
            return Err(value);
        }

        // SAFETY: There are no other borrows at index `head` and the previous
        // value at that index has been dropped or is uninitilized.
        unsafe {
            (&mut *self.buf.get_unchecked(head).get()).write(value);
        }

        // The `Release` operation synchronizes with the `Acquire` operation
        // in `pop`. The `Release` store MUST happen after the data was
        // written.
        self.head.store(new_head, Ordering::Release);
        Ok(())
    }

    /// Pops the oldest element from the queue.
    ///
    /// Returns `None` if the `Queue` is empty.
    pub fn pop(&self) -> Option<T> {
        // The read `tail` is only modified by the thread calling `pop`, therefore
        // we always have the correct value in memory, regardless of memory ordering.
        // Since `Queue` is `!Sync` it is not possible to call `pop` from more than
        // one thread.
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail != head {
            // SAFETY: The `head != tail` indicates that the value at `tail`
            // has been initialized and the buffer is not currently borrowed.
            // at that index.
            let value = unsafe { (&mut *self.buf.get_unchecked(tail).get()).assume_init_read() };

            let new_tail = (tail + 1) % self.buffer_capacity();

            // The `Release` operation synchronizes with the `Acquire` operation
            // in `push`. The `Release` operation MUST happen after the data has been
            // read and reference into the buffer element has been dropped.
            self.tail.store(new_tail, Ordering::Release);
            Some(value)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.saturating_sub(tail)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity of the `Queue`.
    pub fn capacity(&self) -> usize {
        self.buffer_capacity() - 1
    }

    fn buffer_capacity(&self) -> usize {
        self.buf.len()
    }

    /// Splits the `Queue` into `Sender` and `Receiver` which may be moved to different threads.
    pub fn split(self) -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(self);

        (
            Sender {
                inner: inner.clone(),
            },
            Receiver { inner },
        )
    }

    pub fn drain(&self) -> Drain<'_, T> {
        Drain { queue: self }
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

unsafe impl<T> Send for Queue<T> where T: Send {}

#[derive(Debug)]
pub struct Sender<T> {
    inner: Arc<Queue<T>>,
}

impl<T> Sender<T> {
    pub fn push(&mut self, value: T) -> Result<(), T> {
        self.inner.push(value)
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

unsafe impl<T> Send for Sender<T> where T: Send {}

#[derive(Debug)]
pub struct Receiver<T> {
    inner: Arc<Queue<T>>,
}

impl<T> Receiver<T> {
    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain { queue: &self.inner }
    }
}

unsafe impl<T> Send for Receiver<T> where T: Send {}

#[derive(Debug)]
pub struct Drain<'a, T> {
    queue: &'a Queue<T>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.queue.len(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::Queue;

    #[test]
    fn queue_buffer_overflow() {
        let queue = Queue::new(8);
        for _ in 0..queue.capacity() {
            queue.push(vec![0, 1, 2]).unwrap();
        }

        queue.push(vec![0, 1, 2]).unwrap_err();
    }

    #[test]
    fn queue_drop() {
        let queue = Queue::new(8);
        for _ in 0..queue.capacity() {
            queue.push(vec![0, 1, 2]).unwrap();
        }

        drop(queue);
    }

    #[test]
    fn queue_push_pop() {
        let queue = Queue::new(8);
        for _ in 0..queue.capacity() {
            queue.push(vec![0, 1, 2]).unwrap();
        }

        for _ in 0..queue.capacity() {
            queue.pop();
        }
    }

    #[test]
    fn queue_push_pop_interleaved() {
        let queue = Queue::new(8);
        for index in 0..queue.capacity() {
            for _ in 0..index {
                queue.push(vec![0, 1, 2]).unwrap();
            }

            for _ in 0..index {
                queue.pop().unwrap();
            }
        }
    }
}
