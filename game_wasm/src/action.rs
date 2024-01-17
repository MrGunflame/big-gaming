use alloc::vec::Vec;

use crate::components::Decode;
use crate::raw::{action_data_buffer_get, action_data_buffer_len};
use crate::record::RecordReference;

pub(crate) fn action_buffer() -> Vec<u8> {
    unsafe {
        let len = action_data_buffer_len();
        let mut buf = Vec::new();
        action_data_buffer_get(buf.as_mut_ptr());
        buf.set_len(len);
        buf
    }
}

pub struct ActionBuffer {
    buf: Vec<u8>,
}

impl ActionBuffer {
    pub fn load() -> Self {
        Self {
            buf: action_buffer(),
        }
    }

    pub fn get<T>(&self) -> Result<T, T::Error>
    where
        T: Decode,
    {
        T::decode(&self.buf[..])
    }
}

pub trait Action: Decode {
    const ID: RecordReference;
}
