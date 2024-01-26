use alloc::vec::Vec;

use crate::encoding::{decode_fields, Field};
use crate::raw::{host_buffer_get, host_buffer_len};

const HOST_BUFFER_DATA: u32 = 0;
const HOST_BUFFER_PRIMTIVIES: u32 = 1;

pub(crate) fn host_buffer<T>() -> T::Result
where
    T: HostBuffer,
{
    unsafe {
        let len = host_buffer_len(T::INDEX);
        let mut buf = Vec::with_capacity(len);
        host_buffer_get(T::INDEX, buf.as_mut_ptr());
        buf.set_len(len);
        T::from_bytes(buf)
    }
}

/// # Safety
///
/// `INDEX` must be a valid index as handled by the host VM.
pub(crate) unsafe trait HostBuffer {
    const INDEX: u32;

    type Result;

    fn from_bytes(buf: Vec<u8>) -> Self::Result;
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct DataBuffer;

unsafe impl HostBuffer for DataBuffer {
    const INDEX: u32 = HOST_BUFFER_DATA;
    type Result = Vec<u8>;

    fn from_bytes(buf: Vec<u8>) -> Self::Result {
        buf
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct FieldBuffer;

unsafe impl HostBuffer for FieldBuffer {
    const INDEX: u32 = HOST_BUFFER_PRIMTIVIES;
    type Result = Vec<Field>;

    fn from_bytes(buf: Vec<u8>) -> Self::Result {
        decode_fields(&buf)
    }
}
