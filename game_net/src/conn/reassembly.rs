use std::collections::{BTreeMap, BTreeSet};

use crate::proto::sequence::Sequence;
use crate::proto::PacketPosition;

#[derive(Clone, Debug)]
pub(super) struct ReassemblyBuffer {
    fragments: BTreeMap<Sequence, (PacketPosition, Vec<u8>)>,
    start: BTreeSet<Sequence>,
    size: usize,
    max_size: usize,
}

impl ReassemblyBuffer {
    /// Creates a new `ReassemblyBuffer`.
    ///
    /// The `max_size` parameter defines the upper bound of total bytes that can be stored in the
    /// buffer. If `max_size` is `0` only [`PacketPosition::Single`] packets can be processed.
    pub fn new(max_size: usize) -> Self {
        Self {
            fragments: BTreeMap::new(),
            start: BTreeSet::new(),
            max_size,
            size: 0,
        }
    }

    /// Inserts a new segment into the `ReassemblyBuffer` and returns a complete reassembled payload
    /// if possible.
    pub fn insert(
        &mut self,
        seq: Sequence,
        pos: PacketPosition,
        buf: Vec<u8>,
    ) -> Option<(Sequence, Vec<u8>)> {
        match pos {
            PacketPosition::Single => Some((seq, buf)),
            PacketPosition::First | PacketPosition::Middle | PacketPosition::Last => {
                self.insert_fragment(seq, pos, buf);
                self.try_reassemble(seq)
            }
        }
    }

    fn insert_fragment(&mut self, seq: Sequence, pos: PacketPosition, buf: Vec<u8>) {
        // We cannot store the new fragment if it too big for
        // the entire buffer on its own.
        if buf.len() > self.max_size {
            return;
        }

        // Discard the oldest packets until we have enough space to store the new
        // packet.
        loop {
            let is_full = match self.size.checked_add(buf.len()) {
                Some(v) => v > self.max_size,
                None => true,
            };

            if !is_full {
                break;
            }

            // Note that this cannot panic, since at this point `size` would be
            // 0, so `buf.len()` would have to be bigger than `max_size`. This is
            // already checked above.
            let (seq, (pos, bytes)) = self.fragments.pop_first().unwrap();

            self.size -= bytes.len();
            if pos == PacketPosition::First {
                self.start.remove(&seq);
            }
        }

        self.size += buf.len();
        self.fragments.insert(seq, (pos, buf));
        if pos == PacketPosition::First {
            self.start.insert(seq);
        }
    }

    fn try_reassemble(&mut self, seq: Sequence) -> Option<(Sequence, Vec<u8>)> {
        let Some(start) = self.start.range(..=&seq).last().copied() else {
            return None;
        };

        let mut buf = Vec::new();
        let mut seq = start;
        let mut complete = false;
        for (key, (pos, data)) in self.fragments.range(&seq..) {
            if *key != seq {
                return None;
            }

            buf.extend_from_slice(data);
            seq += 1;

            if *pos == PacketPosition::Last {
                complete = true;
                break;
            }
        }

        if !complete {
            return None;
        }

        self.start.remove(&start);
        // Manual iter impl, since we can't impl Step.
        let mut next_seq = start;
        while next_seq != seq {
            self.fragments.remove(&next_seq);
            next_seq += 1;
        }
        self.size -= buf.len();

        Some((start, buf))
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::sequence::Sequence;
    use crate::proto::PacketPosition;

    use super::ReassemblyBuffer;

    #[test]
    fn reassemble_single() {
        let mut buffer = ReassemblyBuffer::new(0);
        let input = vec![0, 1, 2, 3, 4];
        assert_eq!(
            buffer.insert(Sequence::new(0), PacketPosition::Single, input.clone()),
            Some((Sequence::new(0), input))
        );
    }

    #[test]
    fn reassemble_in_order() {
        let mut buffer = ReassemblyBuffer::new(8192);
        assert_eq!(
            buffer.insert(Sequence::new(0), PacketPosition::First, vec![0, 1]),
            None
        );
        assert_eq!(
            buffer.insert(Sequence::new(1), PacketPosition::Middle, vec![2, 3]),
            None
        );
        assert_eq!(
            buffer.insert(Sequence::new(2), PacketPosition::Middle, vec![4, 5]),
            None
        );
        assert_eq!(
            buffer.insert(Sequence::new(3), PacketPosition::Last, vec![6, 7]),
            Some((Sequence::new(0), vec![0, 1, 2, 3, 4, 5, 6, 7]))
        );
    }

    #[test]
    fn reassemble_out_of_order() {
        let mut buffer = ReassemblyBuffer::new(8192);
        assert_eq!(
            buffer.insert(Sequence::new(1), PacketPosition::Middle, vec![2, 3]),
            None
        );
        assert_eq!(
            buffer.insert(Sequence::new(3), PacketPosition::Last, vec![6, 7]),
            None
        );
        assert_eq!(
            buffer.insert(Sequence::new(0), PacketPosition::First, vec![0, 1]),
            None
        );
        assert_eq!(
            buffer.insert(Sequence::new(2), PacketPosition::Middle, vec![4, 5]),
            Some((Sequence::new(0), vec![0, 1, 2, 3, 4, 5, 6, 7]))
        );
    }
}
