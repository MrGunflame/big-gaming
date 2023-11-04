use std::collections::VecDeque;
use std::iter::FusedIterator;

use ahash::HashMap;
use game_common::world::control_frame::ControlFrame;
use game_net::message::DataMessage;

#[derive(Clone, Debug, Default)]
pub struct MessageBacklog {
    snapshots: HashMap<ControlFrame, Snapshot>,
}

impl MessageBacklog {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, cf: ControlFrame, msg: DataMessage) {
        let snapshot = self.snapshots.entry(&cf).or_default();
        snapshot.events.push_back(msg);
    }

    /// Drains the buffer for a specific frame.
    pub fn drain(&mut self, cf: ControlFrame) -> Option<Drain<'_>> {
        let snapshot = self.snapshots.get_mut(&cf)?;

        Some(Drain {
            inner: snapshot.events.drain(..),
        })
    }

    #[inline]
    fn get_index(&self, cf: ControlFrame) -> usize {
        (self.base_cf - cf).0 as usize
    }
}

#[derive(Clone, Debug, Default)]
struct Snapshot {
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
