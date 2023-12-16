use std::iter::FusedIterator;
use std::ops::Range;

use crate::proto::sequence::Sequence;
use crate::proto::Frame;
use crate::request::Request;

/// A `FrameBuffer` contains what frames contained what data.
#[derive(Clone, Debug, Default)]
pub struct FrameBuffer {
    buffer: Vec<Request>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get(&self, index: usize) -> Option<&Request> {
        self.buffer.get(index)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn push(&mut self, req: Request) {
        if cfg!(debug_assertions) {
            if let Some(prev) = self.buffer.last() {
                assert!(req.sequence >= prev.sequence);
            }
        }

        self.buffer.push(req);
    }

    pub fn remove(&mut self, seq: Sequence) {
        // Sequences are guaranteed to be in ascending order.
        // We only need to find the index of the last element
        // containing the sequence.

        // FIXME: Binary search or separate indices may be faster?
        let mut index = 0;
        let mut found = false;
        while index < self.buffer.len() {
            let req = &self.buffer[index];

            if found && req.sequence != seq {
                break;
            } else if req.sequence == seq {
                found = true;
            }

            index += 1;
        }

        self.retain_range(0..index, |_| false);
    }

    pub fn compact(&mut self) -> Vec<Request> {
        let mut removed = vec![];

        let mut index = 0;
        'out: while index < self.buffer.len() {
            let req = &self.buffer[index];

            match &req.frame {
                Frame::EntityDestroy(frame) => {
                    let id = frame.entity;

                    // If we have the `EntityCreate` frame in the buffer
                    // the entity was not yet created on the remote peer
                    // and we can simply remove ALL buffered frames for
                    // that entity.

                    // Note that the `EntityCreate` frame must have come
                    // before the `EntityDestroy` frame, i.e. must be in
                    // range `0..index`.
                    let mut index2 = 0;
                    while index2 < index {
                        let req2 = &self.buffer[index2];

                        index2 += 1;
                    }

                    // If the peer already received the `EntityCreate` frame
                    // the `EntityDestroy` frame needs to be retained, but all
                    // other frames effecting the entity may be removed.
                    let rm = self.retain_range(0..index, |f| f.id() != id);
                    index -= rm.len();
                    removed.extend(rm);

                    // let mut index2 = index + 1;

                    // self.buffer.retain(|(_, frame)| frame.id() != id);
                }
                Frame::EntityTranslate(frame) => {
                    let mut index2 = index + 1;
                    while index2 < self.buffer.len() {
                        let req2 = &self.buffer[index2];

                        if let Frame::EntityTranslate(frame2) = &req2.frame {
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
                        let req2 = &self.buffer[index2];

                        if let Frame::EntityRotate(frame2) = &req2.frame {
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

        removed
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            index: 0,
            buffer: self,
        }
    }

    /// Retains only the elements specified by `f` within the `range`. Returns the number of
    /// removed elements.
    fn retain_range<F>(&mut self, mut range: Range<usize>, mut f: F) -> Vec<Request>
    where
        F: FnMut(&Frame) -> bool,
    {
        debug_assert!(range.start <= range.end);
        debug_assert!(range.end <= self.buffer.len());

        let mut removed = vec![];

        let mut index = range.start;
        while index < range.end {
            let req = &self.buffer[index];

            if f(&req.frame) {
                index += 1;
            } else {
                removed.push(self.buffer.remove(index));
                range.end -= 1;
            }
        }

        removed
    }
}

impl<'a> IntoIterator for &'a FrameBuffer {
    type Item = &'a Request;
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
    type Item = &'a Request;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let req = self.buffer.get(self.index)?;
        self.index += 1;
        Some(req)
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
    use game_common::components::object::ObjectId;
    use game_common::net::ServerEntity;
    use game_common::record::RecordReference;
    use game_common::world::control_frame::ControlFrame;
    use game_common::world::entity::{EntityBody, Object};
    use glam::{Quat, Vec3};

    use crate::proto::sequence::Sequence;
    use crate::proto::{EntityCreate, EntityDestroy, EntityTranslate, Frame, SpawnHost};
    use crate::request::Request;

    use super::FrameBuffer;

    #[test]
    fn test_frame_buffer_compact() {
        let mut buffer = FrameBuffer::new();

        buffer.push(Request {
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            frame: Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(1),
                translation: Vec3::splat(1.0),
            }),
        });

        buffer.push(Request {
            sequence: Sequence::new(1),
            control_frame: ControlFrame(0),
            frame: Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(1),
                translation: Vec3::splat(2.0),
            }),
        });

        buffer.push(Request {
            sequence: Sequence::new(2),
            control_frame: ControlFrame(0),
            frame: Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(2),
                translation: Vec3::splat(3.0),
            }),
        });

        buffer.compact();
        let mut iter = buffer.iter();

        if let Frame::EntityTranslate(frame) = iter.next().unwrap().frame {
            assert_eq!(frame.entity, ServerEntity(1));
            assert_eq!(frame.translation, Vec3::splat(2.0));
        } else {
            panic!();
        }

        if let Frame::EntityTranslate(frame) = iter.next().unwrap().frame {
            assert_eq!(frame.entity, ServerEntity(2));
            assert_eq!(frame.translation, Vec3::splat(3.0));
        } else {
            panic!();
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn frame_buffer_compact_destroy() {
        let mut buffer = FrameBuffer::new();

        buffer.push(Request {
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            frame: Frame::EntityTranslate(EntityTranslate {
                entity: ServerEntity(1),
                translation: Vec3::splat(1.0),
            }),
        });
        buffer.push(Request {
            sequence: Sequence::new(1),
            control_frame: ControlFrame(0),
            frame: Frame::EntityDestroy(EntityDestroy {
                entity: ServerEntity(1),
            }),
        });

        buffer.compact();
        let mut iter = buffer.iter();

        if let Frame::EntityDestroy(frame) = iter.next().unwrap().frame {
            assert_eq!(frame.entity, ServerEntity(1));
        } else {
            panic!();
        }
    }

    // #[test]
    // fn frame_buffer_compact_create_destroy() {
    //     let mut buffer = FrameBuffer::new();

    //     buffer.push(Request {
    //         sequence: Sequence::new(0),
    //         control_frame: ControlFrame(0),
    //         frame: Frame::EntityCreate(EntityCreate {
    //             entity: ServerEntity(1),
    //             translation: Vec3::splat(0.0),
    //             rotation: Quat::IDENTITY,
    //             data: EntityBody::Object(Object {
    //                 id: ObjectId(RecordReference::STUB),
    //             }),
    //         }),
    //     });
    //     buffer.push(Request {
    //         sequence: Sequence::new(1),
    //         control_frame: ControlFrame(0),
    //         frame: Frame::EntityTranslate(EntityTranslate {
    //             entity: ServerEntity(1),
    //             translation: Vec3::splat(1.0),
    //         }),
    //     });
    //     buffer.push(Request {
    //         sequence: Sequence::new(2),
    //         control_frame: ControlFrame(0),
    //         frame: Frame::EntityDestroy(EntityDestroy {
    //             entity: ServerEntity(1),
    //         }),
    //     });

    //     buffer.compact();
    //     let mut iter = buffer.iter();

    //     assert!(iter.next().is_none());
    // }

    #[test]
    fn frame_buffer_remove() {
        let mut buffer = FrameBuffer::new();

        buffer.push(Request {
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            frame: Frame::SpawnHost(SpawnHost {
                entity: ServerEntity(1),
            }),
        });
        buffer.push(Request {
            sequence: Sequence::new(1),
            control_frame: ControlFrame(0),
            frame: Frame::SpawnHost(SpawnHost {
                entity: ServerEntity(1),
            }),
        });
        buffer.push(Request {
            sequence: Sequence::new(1),
            control_frame: ControlFrame(0),
            frame: Frame::SpawnHost(SpawnHost {
                entity: ServerEntity(1),
            }),
        });
        buffer.push(Request {
            sequence: Sequence::new(4),
            control_frame: ControlFrame(0),
            frame: Frame::SpawnHost(SpawnHost {
                entity: ServerEntity(1),
            }),
        });

        buffer.remove(Sequence::new(1));
        assert_eq!(buffer.len(), 1);
    }
}
