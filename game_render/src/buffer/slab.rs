use std::marker::PhantomData;

use bytemuck::NoUninit;
use hashbrown::HashMap;
use slab::Slab;

use crate::api::{Buffer, CommandQueue};
use crate::backend::BufferUsage;

use super::block::BlockBuffer;
use super::GpuBuffer;

/// Index for a [`SlabBuffer`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, NoUninit)]
#[repr(transparent)]
pub struct SlabIndex(u32);

/// A `SlabBuffer` is a dynamic buffer holding any type can be used on the GPU side.
///
/// A `SlabBuffer` is the CPU equivalent of a array of `T` on the GPU side. It allows for
/// insertion and removal while the returned [`SlabIndex`] values remain stable until the value
/// is removed.
#[derive(Debug)]
pub struct SlabBuffer<T> {
    buffer: BlockBuffer,
    _marker: PhantomData<T>,
}

impl<T> SlabBuffer<T>
where
    T: GpuBuffer,
{
    pub fn new(usage: BufferUsage) -> Self {
        Self {
            buffer: BlockBuffer::new(T::pad_to_align(), usage),
            _marker: PhantomData,
        }
    }

    pub fn insert(&mut self, value: &T) -> SlabIndex {
        let index = self.buffer.insert(bytemuck::bytes_of(value));
        SlabIndex(index)
    }

    /// Removes the value at the given `index` from the `SlabBuffer`.
    ///
    /// Note that it is considered invalid to call `remove` with an [`SlabIndex`] that does not
    /// currently reside in the `SlabBuffer`.
    pub fn remove(&mut self, index: SlabIndex) {
        self.buffer.remove(index.0);
    }

    pub fn compact<F>(&mut self, mut rekey: F)
    where
        F: FnMut(SlabIndex, SlabIndex),
    {
        self.buffer
            .compact(|src, dst| rekey(SlabIndex(src), SlabIndex(dst)));
    }

    pub fn buffer(&mut self, queue: &mut CommandQueue<'_>) -> &Buffer {
        self.buffer.buffer(queue)
    }
}

#[derive(Debug)]
pub struct CompactSlabBuffer<T> {
    buffer: SlabBuffer<T>,
    physical_to_logical: HashMap<SlabIndex, u32>,
    logical_to_physical: Slab<SlabIndex>,
}

impl<T> CompactSlabBuffer<T>
where
    T: GpuBuffer,
{
    pub fn new(usage: BufferUsage) -> Self {
        Self {
            buffer: SlabBuffer::new(usage),
            logical_to_physical: Slab::new(),
            physical_to_logical: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: &T) -> SlabIndex {
        let physical_index = self.buffer.insert(value);
        let logical_index = self.logical_to_physical.insert(physical_index) as u32;
        self.physical_to_logical
            .insert(physical_index, logical_index);
        SlabIndex(logical_index)
    }

    pub fn remove(&mut self, index: SlabIndex) {
        self.buffer.remove(index);
        self.buffer.compact(|src, dst| {
            let logical = self.physical_to_logical.remove(&src).unwrap();
            self.physical_to_logical.insert(dst, logical);
            self.logical_to_physical[logical as usize] = dst;
        });
    }

    pub fn buffer(&mut self, queue: &mut CommandQueue<'_>) -> &Buffer {
        self.buffer.buffer(queue)
    }
}
