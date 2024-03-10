use wasmtime::{Caller, Error};

use crate::instance::State;

use super::CallerExt;

pub(super) fn host_buffer_len(caller: Caller<'_, State>, key: u32) -> wasmtime::Result<u32> {
    match &caller.data().as_run()?.get_host_buffer(key) {
        Some(data) => Ok(data.len() as u32),
        None => Err(Error::msg("host buffer not loaded")),
    }
}

pub(super) fn host_buffer_get(
    mut caller: Caller<'_, State>,
    key: u32,
    ptr: u32,
) -> wasmtime::Result<()> {
    // FIXME: We don't have to clone buffer here.
    match caller
        .data()
        .as_run()?
        .get_host_buffer(key)
        .map(|s| s.to_vec())
    {
        Some(data) => caller.write_memory(ptr, &data),
        None => Err(Error::msg("host buffer not loaded")),
    }
}
