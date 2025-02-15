use std::collections::VecDeque;

use crate::proto::sequence::Sequence;

/// A list of lost packets.
#[derive(Clone, Debug)]
pub(super) struct LossList {
    // The list is implemented as a bitmap, where bit 0
    // is `self.base` and bit index is the difference between
    // `self.base` and the new sequence.
    base: Sequence,
    bits: VecDeque<u8>,
    // `true` if at least one sequence is in the set.
    is_init: bool,
    window: u32,
}

impl LossList {
    /// Creates a new `LossList`.
    ///
    /// `window` specifies how many elements can be in the `LossList` at most.
    pub(super) fn new(window: u32) -> Self {
        Self {
            base: Sequence::new(0),
            bits: VecDeque::new(),
            is_init: false,
            window,
        }
    }

    /// Inserts a new [`Sequence`] into the `LossList`.
    ///
    /// The new [`Sequence`] must be greater than any previously inserted sequences (as defined
    /// by the [`Eq`] implementation of [`Sequence`]) and must be less than the oldest in the
    /// `LossList` + `window`.
    pub(super) fn insert(&mut self, seq: Sequence) {
        if self.is_init {
            debug_assert!(seq >= self.base);
            debug_assert!(seq <= self.base + self.window);
        } else {
            debug_assert!(self.bits.is_empty());
        }

        if self.bits.is_empty() {
            self.base = seq;
            self.is_init = true;
        }

        let offset = (seq - self.base.to_bits()).to_bits();
        let index = (offset / 8) as usize;
        let bit = (offset % 8) as u8;

        self.bits.resize(index + 1, 0);
        debug_assert_eq!(self.bits[index] & (1 << bit), 0);
        self.bits[index] |= 1 << bit;
    }

    /// Returns `true` if `seq` was removed.
    pub(super) fn remove(&mut self, seq: Sequence) -> bool {
        let offset = (seq - self.base.to_bits()).to_bits();
        let index = (offset / 8) as usize;
        let bit = (offset % 8) as u8;

        match self.bits.get_mut(index) {
            Some(bits) => {
                let is_set = (*bits & (1 << bit)) != 0;
                *bits &= !(1 << bit);

                while self.bits.front().is_some_and(|bits| *bits == 0) {
                    self.base += 8;
                    self.bits.pop_front();
                }
                self.is_init = !self.bits.is_empty();

                is_set
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::sequence::Sequence;

    use super::LossList;

    #[test]
    fn loss_list_remove() {
        let mut input: Vec<u32> = (0..32).collect();

        let mut list = LossList::new(1024);

        for seq in &input {
            list.insert(Sequence::new(*seq));
        }

        while !input.is_empty() {
            // Always remove the middle element.
            let index = input.len() / 2;
            let value = input.remove(index);

            let res = list.remove(Sequence::new(value));
            assert!(res);
        }
    }

    #[test]
    fn loss_list_wrapping() {
        let mut list = LossList::new(1024);
        list.insert(Sequence::MAX - 1);
        list.insert(Sequence::MAX);
        list.insert(Sequence::MAX + 1);

        assert!(list.remove(Sequence::MAX + 1));
        assert!(list.remove(Sequence::MAX));
        assert!(list.remove(Sequence::MAX - 1));
    }

    #[test]
    fn loss_list_sliding_window() {
        let window = 23;
        let start = Sequence::MAX - 8192;

        let mut list = LossList::new(32);

        for i in 0..window {
            list.insert(start + i);
        }

        for i in 0..u16::MAX as u32 {
            list.insert(start + window + i);
            assert!(list.remove(start + i));
            assert!(list.bits.len() <= 4);
        }
    }
}
