use wasmtime::{Caller, Error};

use crate::instance::State;

use super::CallerExt;

pub(super) fn action_data_buffer_len(caller: Caller<'_, State<'_>>) -> wasmtime::Result<u32> {
    match &caller.data().as_run()?.action_buffer {
        Some(data) => Ok(data.len() as u32),
        None => Err(Error::msg("action data buffer not loaded")),
    }
}

pub(super) fn action_data_buffer_get(
    mut caller: Caller<'_, State<'_>>,
    ptr: u32,
) -> wasmtime::Result<()> {
    match &caller.data().as_run()?.action_buffer {
        Some(data) => caller.write_memory(ptr, &data.clone()),
        None => Err(Error::msg("action data buffer not loaded")),
    }
}
