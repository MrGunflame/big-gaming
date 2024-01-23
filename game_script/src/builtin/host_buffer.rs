use wasmtime::{Caller, Error};

use crate::instance::State;

use super::CallerExt;

pub(super) fn host_buffer_len(caller: Caller<'_, State>, index: u32) -> wasmtime::Result<u32> {
    match &caller.data().as_run()?.host_buffers.get(index as usize) {
        Some(data) => Ok(data.len() as u32),
        None => Err(Error::msg("host buffer not loaded")),
    }
}

pub(super) fn host_buffer_get(
    mut caller: Caller<'_, State>,
    index: u32,
    ptr: u32,
) -> wasmtime::Result<()> {
    match caller
        .data()
        .as_run()?
        .host_buffers
        .get(index as usize)
        .cloned()
    {
        Some(data) => caller.write_memory(ptr, &data),
        None => Err(Error::msg("host buffer not loaded")),
    }
}
