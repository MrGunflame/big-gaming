use game_tracing::trace_span;
use game_wasm::world::RecordReference;
use wasmtime::{Caller, Result};

use crate::events::DispatchEvent;
use crate::instance::State;

use super::CallerExt;

pub fn event_dispatch(
    mut caller: Caller<'_, State>,
    id: u32,
    data_ptr: u32,
    data_len: u32,
    fields_ptr: u32,
    fields_len: u32,
) -> Result<()> {
    let _span = trace_span!("event_dispatch").entered();

    let id: RecordReference = caller.read(id)?;
    let data = caller.read_memory(data_ptr, data_len)?.to_vec();
    let fields = caller.read_memory(fields_ptr, fields_len)?.to_vec();

    caller
        .data_mut()
        .as_run_mut()?
        .events
        .push(DispatchEvent { id, data, fields });

    Ok(())
}
