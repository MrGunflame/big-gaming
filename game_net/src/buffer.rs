use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};
use std::iter::FusedIterator;

use game_common::entity::EntityId;
use glam::{Quat, Vec3};

use crate::proto::sequence::Sequence;
use crate::proto::Frame;
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

/// A `FrameBuffer` contains what frames contained what data.
#[derive(Clone, Debug, Default)]
pub struct FrameBuffer {
    buffer: Vec<(Sequence, Frame)>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get(&self, index: usize) -> Option<&Frame> {
        self.buffer.get(index).map(|(_, f)| f)
    }

    pub fn push(&mut self, seq: Sequence, frame: Frame) {
        if cfg!(debug_assertions) {
            if let Some((prev, _)) = self.buffer.last() {
                assert!(seq >= *prev);
            }
        }

        self.buffer.push((seq, frame));
    }

    pub fn remove(&mut self, seq: Sequence) {
        self.buffer.retain(|(s, _)| *s != seq);
    }

    pub fn shrink(&mut self) {
        let mut index = 0;
        'out: while index < self.buffer.len() {
            let (_, frame) = &self.buffer[index];

            match frame {
                Frame::EntityCreate(_) => (),
                Frame::EntityDestroy(frame) => {
                    let id = frame.entity;

                    self.buffer.retain(|(_, frame)| frame.id() != id);
                }
                Frame::EntityTranslate(frame) => {
                    let mut index2 = index + 1;
                    while index2 < self.buffer.len() {
                        let (_, frame2) = &self.buffer[index2];

                        if let Frame::EntityTranslate(frame2) = frame2 {
                            if index2 > index && frame.entity == frame2.entity {
                                // Replace the current element with the new one, then
                                // remove the current element.
                                self.buffer.swap(index, index2);
                                self.buffer.remove(index2);
                                continue 'out;
                            }
                        }

                        index2 += 1;
                    }
                }
                Frame::EntityRotate(frame) => {
                    let mut index2 = index + 1;
                    while index2 < self.buffer.len() {
                        let (_, frame2) = &self.buffer[index2];

                        if let Frame::EntityRotate(frame2) = frame2 {
                            if index2 > index && frame.entity == frame2.entity {
                                // Replace the current element with the new one, then
                                // remove the current element.
                                self.buffer.swap(index, index2);
                                self.buffer.remove(index2);
                                continue 'out;
                            }
                        }

                        index2 += 1;
                    }
                }
                _ => (),
            }

            index += 1;
        }
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            index: 0,
            buffer: self,
        }
    }
}

impl<'a> IntoIterator for &'a FrameBuffer {
    type Item = &'a Frame;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    index: usize,
    buffer: &'a FrameBuffer,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.buffer.get(self.index)?;
        self.index += 1;
        Some(frame)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.buffer.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

#[cfg(test)]
mod tests {
    use game_common::net::ServerEntity;
    use glam::Vec3;

    use crate::proto::sequence::Sequence;
    use crate::proto::{EntityTranslate, Frame};

    use super::FrameBuffer;

    #[test]
    fn test_frame_buffer_shrink() {
        let mut buffer = FrameBuffer::new();

        buffer.push(
            Sequence::new(0),
            Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(1),
                translation: Vec3::splat(1.0),
            }),
        );

        buffer.push(
            Sequence::new(1),
            Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(1),
                translation: Vec3::splat(2.0),
            }),
        );

        buffer.push(
            Sequence::new(2),
            Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(2),
                translation: Vec3::splat(3.0),
            }),
        );

        buffer.shrink();
        let mut iter = buffer.iter();

        if let Frame::EntityTranslate(frame) = iter.next().unwrap() {
            assert_eq!(frame.entity, ServerEntity(1));
            assert_eq!(frame.translation, Vec3::splat(2.0));
        } else {
            panic!();
        }

        if let Frame::EntityTranslate(frame) = iter.next().unwrap() {
            assert_eq!(frame.entity, ServerEntity(2));
            assert_eq!(frame.translation, Vec3::splat(3.0));
        } else {
            panic!()
        }

        assert!(iter.next().is_none());
    }
}
