use std::marker::PhantomData;

use super::GpuBuffer;

pub struct StableBuffer<T>
where
    T: GpuBuffer,
{
    buffer: Vec<u8>,
    _marker: PhantomData<T>,
    len: u32,
    free_head: Option<u32>,
}

impl<T> StableBuffer<T>
where
    T: GpuBuffer,
{
    pub const fn new() -> Self {
        const { assert!(T::SIZE >= 4) };

        Self {
            buffer: Vec::new(),
            _marker: PhantomData,
            free_head: None,
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    pub fn insert(&mut self, value: &T) -> u32 {
        let value = bytemuck::must_cast_slice(core::slice::from_ref(value));

        match self.free_head {
            Some(index) => {
                let offset = T::SIZE * index as usize;
                let buf = &mut self.buffer[offset..offset + T::SIZE];

                let next_free = u32::from_ne_bytes(buf[0..4].try_into().unwrap());
                self.free_head = if next_free == 0 {
                    None
                } else {
                    Some(next_free)
                };

                buf.copy_from_slice(value);

                index
            }
            None => {
                self.buffer.extend(value);

                let align_bytes = T::ALIGN % T::SIZE;
                for _ in 0..align_bytes {
                    self.buffer.push(0);
                }

                let key = self.len;
                self.len += 1;
                key
            }
        }
    }

    pub fn remove(&mut self, key: u32) {
        self.len -= 1;

        let offset = T::SIZE * key as usize;

        let buf = &mut self.buffer[offset..offset + T::SIZE];

        let next_free = self.free_head.unwrap_or(0);
        buf[0..4].copy_from_slice(&next_free.to_ne_bytes());

        self.free_head = Some(key);
    }
}
