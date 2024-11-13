use std::mem::MaybeUninit;

pub(crate) struct RingBuf<T> {
    buf: Box<[T]>,
    head: usize,
    tail: usize,
}

impl<T> RingBuf<T> {
    pub(crate) fn new() -> Self {
        Self {
            buf: Box::new([]),
            head: 0,
            tail: 0,
        }
    }

    pub(crate) fn resize(&mut self, new_len: usize, value: T)
    where
        T: Copy,
    {
        let mut new_buf = Vec::with_capacity(new_len);
        let old_buf = core::mem::take(&mut self.buf);

        if new_len >= self.buf.len() {
            let count = new_len - old_buf.len();

            // Copy all old elements.
            for elem in old_buf.into_vec() {
                new_buf.push(elem);
            }

            // Fill the remaining slots with `value`.
            for _ in 0..count {
                new_buf.push(value);
            }
        } else {
            let count = old_buf.len() - new_len;

            for elem in old_buf.into_vec().into_iter().take(count) {
                new_buf.push(elem);
            }
        }

        self.buf = new_buf.into_boxed_slice();
    }
}
