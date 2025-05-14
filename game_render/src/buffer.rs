pub mod block;
pub mod slab;
pub mod suballocated;

use std::marker::PhantomData;

#[cfg(target_endian = "big")]
compile_error!("`DynamicBuffer` doesn't support big endian targets");

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Vec3};

use crate::api::Buffer;
use crate::backend::IndexFormat;

#[derive(Debug)]
pub struct IndexBuffer {
    pub buffer: Buffer,
    pub format: IndexFormat,
    /// Length of the buffer, in elements.
    pub len: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Vec3F32 {
    x: f32,
    y: f32,
    z: f32,
    _pad: u32,
}

impl From<Vec3> for Vec3F32 {
    fn from(vec: Vec3) -> Self {
        Self {
            x: vec.x,
            y: vec.y,
            z: vec.z,
            _pad: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Mat3F32 {
    x_axis: [f32; 3],
    _pad0: u32,
    y_axis: [f32; 3],
    _pad1: u32,
    z_axis: [f32; 3],
    _pad2: u32,
}

impl From<Mat3> for Mat3F32 {
    fn from(mat: Mat3) -> Self {
        Self {
            x_axis: mat.x_axis.to_array(),
            y_axis: mat.y_axis.to_array(),
            z_axis: mat.z_axis.to_array(),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

pub trait GpuBuffer: Zeroable + Pod {
    const SIZE: usize;
    const ALIGN: usize;

    /// Returns the size of `Self` aligned to `Self::ALIGN`.
    fn pad_to_align() -> usize {
        // Implementation from `Layout::pad_to_align`:
        // https://doc.rust-lang.org/stable/src/core/alloc/layout.rs.html#333-342
        let align_m1 = Self::ALIGN - 1;
        let size_rounded_up = (Self::SIZE + align_m1) & !align_m1;
        size_rounded_up
    }
}

///
/// ```text
/// struct B {
///     // Aligned to `T`.
///     count: u32,
///     // Contains at least one element.
///     elems: array<T>,
/// }
#[derive(Debug)]
pub struct DynamicBuffer<T>
where
    T: GpuBuffer,
{
    buf: Vec<u8>,
    _marker: PhantomData<T>,
    cap: u32,
}

impl<T> DynamicBuffer<T>
where
    T: GpuBuffer,
{
    pub fn new() -> Self {
        //
        let buf = vec![
            0;
            Self::counter_offset() // Memory for counter
            + T::SIZE + Self::padding_needed() // Memory for first element
        ];

        Self {
            buf,
            _marker: PhantomData,
            cap: 1,
        }
    }

    /// Returns the number of elements in this buffer.
    pub fn len(&self) -> u32 {
        let bytes = &self.buf[0..4];
        u32::from_ne_bytes(bytes.try_into().unwrap())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, item: T) {
        let index = self.len();

        if self.cap <= self.len() {
            self.reserve(1);
        }

        let slice = self.get_memory_mut(index);
        slice.copy_from_slice(bytemuck::bytes_of(&item));

        self.set_len(self.len() + 1);
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn set_len(&mut self, len: u32) {
        let bytes = &mut self.buf[0..4];
        bytes.copy_from_slice(&len.to_ne_bytes());
    }

    fn reserve(&mut self, additional: usize) {
        let pad = Self::padding_needed();
        let size = T::SIZE + pad;

        self.buf.resize(self.buf.len() + size * additional, 0);
        self.cap += additional as u32;
    }

    fn padding_needed() -> usize {
        (T::SIZE.wrapping_add(T::ALIGN).wrapping_sub(1) & !T::ALIGN.wrapping_sub(1))
            .wrapping_sub(T::SIZE)
    }

    fn get_memory_mut(&mut self, index: u32) -> &mut [u8] {
        let size = T::SIZE + Self::padding_needed();

        let start = Self::counter_offset() + size * index as usize;
        let end = start + size;

        &mut self.buf[start..end]
    }

    fn counter_offset() -> usize {
        T::ALIGN.max(size_of::<u32>())
    }
}

impl<T> Default for DynamicBuffer<T>
where
    T: GpuBuffer,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Extend<T> for DynamicBuffer<T>
where
    T: GpuBuffer,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in iter.into_iter() {
            self.push(elem);
        }
    }
}

impl<T> FromIterator<T> for DynamicBuffer<T>
where
    T: GpuBuffer,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut buffer = Self::new();
        buffer.extend(iter);
        buffer
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::{Pod, Zeroable};

    use super::{DynamicBuffer, GpuBuffer};

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
    #[repr(C)]
    struct TestStruct {
        a: [u8; 3],
        b: u8,
        c: [u8; 3],
        d: u8,
        e: u8,
        f: u8,
        g: [u8; 2],
        h: [u8; 4],
    }

    impl GpuBuffer for TestStruct {
        const SIZE: usize = size_of::<Self>();
        const ALIGN: usize = 16;
    }

    #[test]
    fn dynamic_buffer_push_once() {
        let mut buffer = DynamicBuffer::new();
        buffer.push(TestStruct {
            a: [1, 2, 3],
            b: 4,
            c: [5, 6, 7],
            d: 8,
            e: 9,
            f: 10,
            g: [11, 12],
            h: [13, 14, 15, 16],
        });

        assert_eq!(
            buffer.as_bytes(),
            [
                1, 0, 0, 0, // count
                0, 0, 0, 0, // align
                0, 0, 0, 0, // align
                0, 0, 0, 0, // align
                1, 2, 3, 4, // a + b
                5, 6, 7, 8, // c + d
                9, 10, 11, 12, // e + f + g
                13, 14, 15, 16, // h
            ]
        );
    }

    #[test]
    fn dynamic_buffer_push_many() {
        let mut buffer = DynamicBuffer::new();

        for _ in 0..3 {
            buffer.push(TestStruct {
                a: [1, 2, 3],
                b: 4,
                c: [5, 6, 7],
                d: 8,
                e: 9,
                f: 10,
                g: [11, 12],
                h: [13, 14, 15, 16],
            });
        }

        assert_eq!(
            buffer.as_bytes(),
            [
                3, 0, 0, 0, // count
                0, 0, 0, 0, // align
                0, 0, 0, 0, // align
                0, 0, 0, 0, // align
                1, 2, 3, 4, // a + b
                5, 6, 7, 8, // c + d
                9, 10, 11, 12, // e + f + g
                13, 14, 15, 16, // h
                1, 2, 3, 4, // a + b
                5, 6, 7, 8, // c + d
                9, 10, 11, 12, // e + f + g
                13, 14, 15, 16, // h
                1, 2, 3, 4, // a + b
                5, 6, 7, 8, // c + d
                9, 10, 11, 12, // e + f + g
                13, 14, 15, 16, // h
            ]
        );
    }
}
