use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use crate::api::{Buffer, BufferDescriptor, CommandQueue};
use crate::backend::allocator::UsageFlags;
use crate::backend::BufferUsage;

/// A buffer for storing fixed size blocks, backed by GPU memory.
#[derive(Debug)]
pub struct BlockBuffer {
    buffer: Option<Buffer>,
    buffer_size: usize,
    block_size: usize,
    occupied_blocks: BTreeSet<u32>,
    free_blocks: BinaryHeap<Reverse<u32>>,
    writes_queued: Vec<BufferWrite>,
    usage: BufferUsage,
}

impl BlockBuffer {
    pub fn new(block_size: usize, usage: BufferUsage) -> Self {
        Self {
            buffer: None,
            block_size,
            free_blocks: BinaryHeap::new(),
            buffer_size: 0,
            usage: usage | BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
            occupied_blocks: BTreeSet::new(),
            writes_queued: Vec::new(),
        }
    }

    pub fn insert(&mut self, bytes: &[u8]) -> u32 {
        assert!(bytes.len() <= self.block_size);

        if self.free_blocks.is_empty() {
            self.grow();
        }

        debug_assert!(!self.free_blocks.is_empty());
        let index = self.free_blocks.pop().unwrap().0 as u32;

        self.occupied_blocks.insert(index);
        self.writes_queued
            .push(BufferWrite::Data(index, bytes.to_vec()));

        index
    }

    fn grow(&mut self) {
        let old_size = self.buffer_size;
        let new_size = self.buffer_size.max(1) << 1;
        self.buffer_size = new_size;
        assert!(new_size <= u32::MAX as usize);

        for index in old_size..new_size {
            self.free_blocks.push(Reverse(index as u32));
        }

        if self.buffer.is_some()
            && !matches!(self.writes_queued.get(0), Some(BufferWrite::BufferExpand))
        {
            self.writes_queued.insert(0, BufferWrite::BufferExpand);
        }
    }

    pub fn remove(&mut self, index: u32) {
        assert!(self.occupied_blocks.contains(&index));
        self.occupied_blocks.remove(&index);
        self.free_blocks.push(Reverse(index));
    }

    /// Compact the buffer as much as possible by moving elements to the start of the buffer.
    pub fn compact<F>(&mut self, mut rekey: F)
    where
        F: FnMut(u32, u32),
    {
        let mut moved_indices = Vec::new();
        for index in self.occupied_blocks.iter().rev() {
            let Some(next_free) = self.free_blocks.peek() else {
                break;
            };

            if next_free.0 > *index {
                break;
            }

            let next_free = self.free_blocks.pop().unwrap();
            rekey(*index, next_free.0);

            self.writes_queued.push(BufferWrite::BufferCopy {
                src: *index,
                dst: next_free.0,
            });
            moved_indices.push((*index, next_free.0));
            self.free_blocks.push(Reverse(*index));
        }

        for (src, dst) in moved_indices {
            self.occupied_blocks.remove(&src);
            self.occupied_blocks.insert(dst);
        }
    }

    pub fn buffer(&mut self, queue: &mut CommandQueue<'_>) -> &Buffer {
        let buffer = self.buffer.get_or_insert_with(|| {
            let size = (self.block_size * self.buffer_size.max(1)) as u64;

            queue.create_buffer(&BufferDescriptor {
                flags: UsageFlags::empty(),
                usage: self.usage,
                size,
            })
        });

        for cmd in self.writes_queued.drain(..) {
            match cmd {
                BufferWrite::BufferExpand => {
                    let size = (self.block_size * self.buffer_size) as u64;

                    let new_buffer = queue.create_buffer(&BufferDescriptor {
                        size,
                        usage: self.usage,
                        flags: UsageFlags::empty(),
                    });

                    queue
                        .copy_buffer_to_buffer(buffer.slice(..), new_buffer.slice(..buffer.size()));
                    *buffer = new_buffer;
                }
                BufferWrite::BufferCopy { src, dst } => {
                    let src_start = src as u64 * self.block_size as u64;
                    let src_end = (src as u64 + 1) * self.block_size as u64;
                    let dst_start = dst as u64 * self.block_size as u64;
                    let dst_end = (dst as u64 + 1) * self.block_size as u64;

                    queue.copy_buffer_to_buffer(
                        buffer.slice(src_start..src_end),
                        buffer.slice(dst_start..dst_end),
                    );
                }
                BufferWrite::Data(index, bytes) => {
                    let offset = self.block_size as u64 * index as u64;
                    queue.write_buffer(buffer.slice(offset..offset + bytes.len() as u64), &bytes);
                }
            }
        }

        &*buffer
    }
}

#[derive(Clone, Debug, PartialEq)]
enum BufferWrite {
    BufferExpand,
    BufferCopy { src: u32, dst: u32 },
    Data(u32, Vec<u8>),
}

#[cfg(test)]
mod tests {
    use crate::backend::BufferUsage;

    use super::{BlockBuffer, BufferWrite};

    #[test]
    fn block_buffer_compact() {
        let mut buffer = BlockBuffer::new(4, BufferUsage::empty());
        let i0 = buffer.insert(&[1, 2, 3, 4]);
        let i1 = buffer.insert(&[1, 2, 3, 4]);
        let i2 = buffer.insert(&[1, 2, 3, 4]);
        buffer.remove(i1);

        buffer.writes_queued.clear();
        buffer.compact(|src, dst| {
            assert_eq!(src, i2);
            assert_eq!(dst, i1);
        });

        assert_eq!(
            buffer.writes_queued,
            [BufferWrite::BufferCopy { src: i2, dst: i1 }]
        );

        buffer.remove(i0);

        buffer.writes_queued.clear();
        buffer.compact(|src, dst| {
            assert_eq!(src, i1);
            assert_eq!(dst, i0);
        });

        assert_eq!(
            buffer.writes_queued,
            [BufferWrite::BufferCopy { src: i1, dst: i0 }]
        );
    }
}
