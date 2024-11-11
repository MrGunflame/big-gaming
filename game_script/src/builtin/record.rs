use game_common::record::RecordReference;
use game_data::record::RecordKind;
use game_tracing::trace_span;
use game_wasm::raw::record::RawRecordFilter;
use game_wasm::raw::{RESULT_NO_RECORD, RESULT_OK};
use game_wasm::record::ModuleId;
use wasmtime::{Caller, Result};

use crate::builtin::{AsMemory, CallerExt};
use crate::instance::State;

pub fn record_list_count(mut caller: Caller<'_, State>, filter: u32, out: u32) -> Result<u32> {
    let _span = trace_span!("record_list_count").entered();

    let raw_filter: RawRecordFilter = caller.read(filter)?;

    let mut filter = RecordFilter::default();
    if raw_filter.filter_module != 0 {
        filter.module = Some(raw_filter.module);
    }

    if raw_filter.filter_kind != 0 {
        filter.kind = Some(RecordKind(raw_filter.kind));
    }

    let count = caller
        .data()
        .as_run()?
        .records()
        .iter()
        .filter(|(module, record)| {
            if let Some(filtered) = filter.module {
                if filtered != *module {
                    return false;
                }
            }

            if let Some(filtered) = filter.kind {
                if filtered != record.kind {
                    return false;
                }
            }

            true
        })
        .count() as u32;

    caller.write(out, &count)?;
    Ok(RESULT_OK)
}

pub fn record_list_copy(
    mut caller: Caller<'_, State>,
    filter: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("record_list_copy").entered();

    let raw_filter: RawRecordFilter = caller.read(filter)?;

    let mut filter = RecordFilter::default();
    if raw_filter.filter_module != 0 {
        filter.module = Some(raw_filter.module);
    }

    if raw_filter.filter_kind != 0 {
        filter.kind = Some(RecordKind(raw_filter.kind));
    }

    let (mut memory, data) = caller.split()?;

    let iter = data
        .as_run()?
        .records()
        .iter()
        .filter_map(|(module, record)| {
            if let Some(filtered) = filter.module {
                if filtered != module {
                    return None;
                }
            }

            if let Some(filtered) = filter.kind {
                if filtered != record.kind {
                    return None;
                }
            }

            Some(RecordReference {
                module,
                record: record.id,
            })
        });

    memory.write_iter(out, iter.take(len as usize))?;
    Ok(RESULT_OK)
}

/// ```no_run
/// # use game_common::record::RecordReference;
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

    caller.write::<u32>(out, &(record.data.len() as u32))?;
    Ok(RESULT_OK)
}

///```no_run
/// # use game_common::record::RecordReference;
/// # extern "C" {
/// fn record_data_copy(id: *const RecordReference, dst: *mut u8, len: usize) -> u32;
/// }
/// ```
pub fn record_data_copy(mut caller: Caller<'_, State>, id: u32, dst: u32, len: u32) -> Result<u32> {
    let _span = trace_span!("record_data_copy").entered();
    tracing::trace!("record_data_copy(id={}, dst={}, len={})", id, dst, len);

    let (mut memory, data) = caller.split()?;

    let id: RecordReference = memory.read(id)?;
    let Some(record) = data.as_run()?.records().get(id) else {
        return Ok(RESULT_NO_RECORD);
    };

    let count = std::cmp::min(record.data.len(), len as usize);

    memory.write_memory(dst, &record.data[..count])?;
    Ok(RESULT_OK)
}

#[derive(Copy, Clone, Debug, Default)]
struct RecordFilter {
    module: Option<ModuleId>,
    kind: Option<RecordKind>,
}
