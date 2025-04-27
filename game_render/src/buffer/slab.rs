use std::marker::PhantomData;

use bytemuck::NoUninit;

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
#[derive(Clone, Debug, Default)]
pub struct SlabBuffer<T> {
    bytes: Vec<u8>,
    free_list: Vec<u32>,
    _marker: PhantomData<T>,
}

impl<T> SlabBuffer<T>
where
    T: GpuBuffer,
{
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            free_list: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn insert(&mut self, value: &T) -> SlabIndex {
        let index = match self.free_list.pop() {
            Some(index) => index as usize,
            None => {
                let index = self.bytes.len() / T::pad_to_align();
                self.bytes.resize((index + 1) * T::pad_to_align(), 0);
                index
            }
        };

        let offset = index * T::pad_to_align();

        debug_assert!(self.bytes.len() >= offset + T::SIZE);
        self.bytes[offset..offset + T::SIZE].copy_from_slice(bytemuck::bytes_of(value));

        SlabIndex(index as u32)
    }

    /// Removes the value at the given `index` from the `SlabBuffer`.
    ///
    /// Note that it is considered invalid to call `remove` with an [`SlabIndex`] that does not
    /// currently reside in the `SlabBuffer`.
    pub fn remove(&mut self, index: SlabIndex) {
        self.free_list.push(index.0 as u32);
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
