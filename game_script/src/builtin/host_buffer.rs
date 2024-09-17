use game_tracing::trace_span;
use wasmtime::{Caller, Error};

use crate::instance::State;

use super::{AsMemory, CallerExt};

pub(super) fn host_buffer_len(caller: Caller<'_, State>, key: u32) -> wasmtime::Result<u32> {
    let _span = trace_span!("host_buffer_len").entered();

    match caller.data().as_run()?.get_host_buffer(key) {
        Some(data) => Ok(data.len() as u32),
        None => Err(Error::msg("host buffer not loaded")),
    }
}

pub(super) fn host_buffer_get(
    mut caller: Caller<'_, State>,
    key: u32,
    ptr: u32,
) -> wasmtime::Result<()> {
    let _span = trace_span!("host_buffer_get").entered();

    let (mut memory, data) = caller.split()?;

    match data.as_run()?.get_host_buffer(key) {
        Some(data) => memory.write_memory(ptr, &data),
        None => Err(Error::msg("host buffer not loaded")),
    }
}
