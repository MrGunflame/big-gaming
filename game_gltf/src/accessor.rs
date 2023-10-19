use std::marker::PhantomData;

pub type Positions = [f32; 3];
pub type Normals = [f32; 3];
pub type Tangents = [f32; 4];
pub type Uvs = [f32; 2];

use bytes::Buf;
use gltf::Accessor;

use crate::{EofError, GltfStagingData};

pub trait Item: Sized + Copy {
    fn from_slice(buf: &[u8]) -> Self;

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool;
}

impl Item for u8 {
    fn from_slice(buf: &[u8]) -> Self {
        buf[0]
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => self >= min && self <= max,
            (Some(min), None) => self >= min,
            (None, Some(max)) => self <= max,
            (None, None) => true,
        }
    }
}

impl Item for u16 {
    fn from_slice(mut buf: &[u8]) -> Self {
        buf.get_u16_le()
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => self >= min && self <= max,
            (Some(min), None) => self >= min,
            (None, Some(max)) => self <= max,
            (None, None) => true,
        }
    }
}

impl Item for u32 {
    fn from_slice(mut buf: &[u8]) -> Self {
        buf.get_u32_le()
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => self >= min && self <= max,
            (Some(min), None) => self >= min,
            (None, Some(max)) => self <= max,
            (None, None) => true,
        }
    }
}

impl Item for f32 {
    fn from_slice(mut buf: &[u8]) -> Self {
        buf.get_f32_le()
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => self >= min && self <= max,
            (Some(min), None) => self >= min,
            (None, Some(max)) => self <= max,
            (None, None) => true,
        }
    }
}

impl<T> Item for [T; 1]
where
    T: Item,
{
    fn from_slice(buf: &[u8]) -> Self {
        [T::from_slice(buf)]
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (Some(min), None) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), None) {
                        return false;
                    }
                }

                true
            }
            (None, Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(None, Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (None, None) => true,
        }
    }
}

impl<T> Item for [T; 2]
where
    T: Item,
{
    fn from_slice(buf: &[u8]) -> Self {
        [
            T::from_slice(buf),
            T::from_slice(&buf[std::mem::size_of::<T>()..]),
        ]
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (Some(min), None) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), None) {
                        return false;
                    }
                }

                true
            }
            (None, Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(None, Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (None, None) => true,
        }
    }
}

impl<T> Item for [T; 3]
where
    T: Item,
{
    fn from_slice(buf: &[u8]) -> Self {
        [
            T::from_slice(buf),
            T::from_slice(&buf[std::mem::size_of::<T>()..]),
            T::from_slice(&buf[2 * std::mem::size_of::<T>()..]),
        ]
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (Some(min), None) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), None) {
                        return false;
                    }
                }

                true
            }
            (None, Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(None, Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (None, None) => true,
        }
    }
}

impl<T> Item for [T; 4]
where
    T: Item,
{
    fn from_slice(buf: &[u8]) -> Self {
        [
            T::from_slice(buf),
            T::from_slice(&buf[std::mem::size_of::<T>()..]),
            T::from_slice(&buf[2 * std::mem::size_of::<T>()..]),
            T::from_slice(&buf[3 * std::mem::size_of::<T>()..]),
        ]
    }

    fn is_valid(self, min: Option<Self>, max: Option<Self>) -> bool {
        match (min, max) {
            (Some(min), Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (Some(min), None) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(Some(min[index]), None) {
                        return false;
                    }
                }

                true
            }
            (None, Some(max)) => {
                for (index, elem) in self.iter().enumerate() {
                    if !elem.is_valid(None, Some(max[index])) {
                        return false;
                    }
                }

                true
            }
            (None, None) => true,
        }
    }
}

pub struct ItemReader<'a, T>
where
    T: Item,
{
    stride: usize,
    buffer: &'a [u8],
    _marker: PhantomData<fn() -> T>,
}

impl<'a, T> ItemReader<'a, T>
where
    T: Item,
{
    pub(crate) fn new(
        semantic: &'static str,
        accessor: &Accessor,
        data: &'a GltfStagingData,
    ) -> Result<Self, EofError> {
        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let buffer = data
            .buffer(buffer.source(), view.offset(), view.length())
            .unwrap();

        let stride = view.stride().unwrap_or(std::mem::size_of::<T>());

        let start = accessor.offset();
        let end = start + stride * (accessor.count() - 1) + std::mem::size_of::<T>();

        let Some(slice) = buffer.get(start..end) else {
            let bytes_required = end - start;
            let bytes_avail = buffer.len();

            return Err(EofError {
                semantic,
                bytes_avail,
                bytes_required,
            });
        };

        Ok(Self {
            stride,
            buffer: slice,
            _marker: PhantomData,
        })
    }
}

impl<'a, T> Iterator for ItemReader<'a, T>
where
    T: Item,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let stride = if self.buffer.len() >= self.stride {
            self.stride
        } else if self.buffer.len() >= std::mem::size_of::<T>() {
            std::mem::size_of::<T>()
        } else {
            return None;
        };

        let (item, buffer) = self.buffer.split_at(stride);
        self.buffer = buffer;
        Some(T::from_slice(item))
    }
}
