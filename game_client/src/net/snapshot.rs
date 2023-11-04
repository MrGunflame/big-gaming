use std::collections::VecDeque;
use std::iter::FusedIterator;

use game_common::world::control_frame::ControlFrame;
use game_net::message::DataMessage;

#[derive(Clone, Debug, Default)]
pub struct MessageBacklog {
    snapshots: Box<[Snapshot]>,
    tail: ControlFrame,
}

impl MessageBacklog {
    pub fn new(size: usize) -> Self {
        let mut snapshots = Vec::with_capacity(size);
        for _ in 0..size {
            snapshots.push(Snapshot::default());
        }

        Self {
            snapshots: snapshots.into_boxed_slice(),
            tail: ControlFrame(0),
        }
    }

    pub fn insert(&mut self, cf: ControlFrame, msg: DataMessage) {
        // If the message if older than the last consumed message,
        // enqueue it for the oldest available snapshot.
        if cf < self.tail {
            let snapshot = &mut self.snapshots[self.tail.0 as usize % self.snapshots.len()];
            snapshot.events.push_front(msg);
            snapshot.has_data = true;
            return;
        }

        let index = cf.0 as usize % self.snapshots.len();
        let snapshot = &mut self.snapshots[index];
        snapshot.events.push_back(msg);
        snapshot.has_data = true;
    }

    /// Drains the buffer for a specific frame.
    pub fn drain(&mut self, cf: ControlFrame) -> Option<Drain<'_>> {
        self.tail = cf;
        let index = cf.0 as usize % self.snapshots.len();

        let snapshot = &mut self.snapshots[index];
        // FIXME: Should we just return an empty iterator instead?
        if !snapshot.has_data {
            return None;
        }

        Some(Drain {
            inner: snapshot.events.drain(..),
        })
    }
}

#[derive(Clone, Debug, Default)]
struct Snapshot {
    has_data: bool,
    events: VecDeque<DataMessage>,
}

#[derive(Debug)]
pub struct Drain<'a> {
    inner: std::collections::vec_deque::Drain<'a, DataMessage>,
}

impl<'a> Iterator for Drain<'a> {
    type Item = DataMessage;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for Drain<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> FusedIterator for Drain<'a> {}
