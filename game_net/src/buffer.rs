use std::borrow::Borrow;
use std::collections::VecDeque;

use game_common::entity::EntityId;
use glam::{Quat, Vec3};

use crate::snapshot::SnapshotId;

/// Constant time replay buffer
#[derive(Clone, Debug)]
pub struct ReplayBuffer {
    id: EntityId,
    buf: VecDeque<(SnapshotId, ReplayData)>,
    limit: usize,
    /// The next snapshot
    head: SnapshotId,
}

impl ReplayBuffer {
    pub fn new(id: EntityId) -> Self {
        Self {
            id,
            buf: VecDeque::new(),
            limit: 120,
            head: SnapshotId(0),
        }
    }

    #[inline]
    pub fn id(&self) -> EntityId {
        self.id
    }

    pub fn push(&mut self, id: SnapshotId, data: ReplayData) {
        #[cfg(debug_assertions)]
        if let Some((last, _)) = self.buf.back() {
            assert!(id > *last);
        }

        self.buf.push_back((id, data));
        if self.buf.len() > self.limit {
            self.buf.pop_front();
        }
    }

    pub fn get<T>(&self, id: T) -> Option<&ReplayData>
    where
        T: Borrow<SnapshotId>,
    {
        self.buf
            .iter()
            .find(|(i, _)| i == id.borrow())
            .map(|(_, d)| d)
    }

    // pub fn next(&mut self) -> Option<NextReplayData<'_>> {
    //     // let id = self.buf.get(self.head)?;

    //     // // Wrap the head at the buffer capacity.
    //     // self.head += 1;
    //     // if self.head >= self.limit {
    //     //     self.head = 0;
    //     // }

    //     // self.head += 1;

    //     let id = self.head;
    //     self.head += 1;

    //     let mut delta = 1;
    //     loop {
    //         if let Some(data) = self.get(id) {
    //             return Some(NextReplayData { delta, data });
    //         }

    //         delta += 1;
    //         self.head += 1;
    //     }
    // }

    pub fn seek(&mut self) {}
}

#[derive(Copy, Clone, Debug)]
pub struct NextReplayData<'a> {
    /// Delta from the previous snapshot.
    ///
    /// This is usually `1` (the next snapshot), but may be higher if a snapshot was lost.
    /// `delta * server_tickrate` is the interpolation period.
    pub delta: u32,
    pub data: &'a ReplayData,
}

#[derive(Clone, Debug)]
pub struct ReplayData {
    pub translation: Vec3,
    pub rotation: Quat,
}
