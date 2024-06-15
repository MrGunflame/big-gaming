use game_common::record::RecordReference;
use game_tracing::trace_span;
use game_wasm::raw::{RESULT_NO_RECORD, RESULT_OK};
use wasmtime::{Caller, Result};

use crate::builtin::CallerExt;
use crate::instance::State;

/// ```no_run
/// # extern "C" {
/// fn record_data_len(id: *const RecordReference, out: *mut usize) -> u32;
/// # }
/// ```
pub fn record_data_len(mut caller: Caller<'_, State>, id: u32, out: u32) -> Result<u32> {
    let _span = trace_span!("record_data_len").entered();
    tracing::trace!("record_data_len(id={}, out={})", id, out);

    let id: RecordReference = caller.read(id)?;
    let Some(record) = caller.data().as_run()?.records().get(id) else {
        return Ok(RESULT_NO_RECORD);
    };

    caller.write::<u32>(out, &(record.data.len() as u32));
    Ok(RESULT_OK)
}

///```no_run
/// # extern "C" {
/// fn record_data_copy(id: *const RecordReference, dst: *mut u8, len: usize) -> u32;
/// }
/// ```
pub fn record_data_copy(mut caller: Caller<'_, State>, id: u32, dst: u32, len: u32) -> Result<u32> {
    let _span = trace_span!("record_data_copy").entered();
    tracing::trace!("record_data_copy(id={}, dst={}, len={})", id, dst, len);

    let id: RecordReference = caller.read(id)?;
    // TODO: REMOVE CLONE.
    let Some(record) = caller.data().as_run()?.records().get(id).cloned() else {
        return Ok(RESULT_NO_RECORD);
    };

    let count = std::cmp::min(record.data.len(), len as usize);

    caller.write_memory(dst, &record.data[..count])?;
    Ok(RESULT_OK)
}
