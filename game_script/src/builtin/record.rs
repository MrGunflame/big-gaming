use game_common::record::RecordReference;
use game_data::record::RecordKind;
use game_tracing::trace_span;
use game_wasm::raw::record::{Record, RecordKind as RawRecordKind};
use wasmtime::{Caller, Result};

use crate::builtin::CallerExt;
use crate::instance::State;

pub fn get_record(mut caller: Caller<'_, State<'_>>, record_id: u32, out: u32) -> Result<u32> {
    let _span = trace_span!("get_record").entered();
    tracing::trace!("get_record(record_id={}, out={}", record_id, out);

    let id: RecordReference = caller.read(record_id)?;
    let Some(record) = caller.data().as_run()?.records.get(id) else {
        return Ok(1);
    };

    let kind = match record.kind() {
        RecordKind::Item => RawRecordKind::ITEM,
        RecordKind::Object => RawRecordKind::OBJECT,
        RecordKind::Race => RawRecordKind::RACE,
        _ => todo!(),
    };

    caller.write(out, &Record { kind })?;
    Ok(0)
}

pub fn get_record_len_component(
    mut caller: Caller<'_, State<'_>>,
    record_id: u32,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("get_record_len_component").entered();
    tracing::trace!(
        "get_record_len_component(record_id={}, out={})",
        record_id,
        out
    );

    let id: RecordReference = caller.read(record_id)?;
    let Some(record) = caller.data().as_run()?.records.get(id) else {
        return Ok(1);
    };

    caller.write(out, &(record.components.len() as u32))?;
    Ok(0)
}

pub fn get_record_component_keys(
    mut caller: Caller<'_, State<'_>>,
    record_id: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("get_record_component_keys").entered();
    tracing::trace!(
        "get_record_component_keys(record_id={}, out={}, len={})",
        record_id,
        out,
        len
    );

    let id: RecordReference = caller.read(record_id)?;
    let Some(record) = caller.data().as_run()?.records.get(id) else {
        return Ok(1);
    };

    for (index, component) in record.components.iter().enumerate() {
        let ptr = out + (std::mem::size_of::<RecordReference>() * index) as u32;
        caller.write(ptr, &component.id)?;
    }

    Ok(0)
}

pub fn get_record_component_len(
    mut caller: Caller<'_, State<'_>>,
    record_id: u32,
    component_id: u32,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("get_record_component_len").entered();
    tracing::trace!(
        "get_record_component_len(record_id={}, component_id={}, out={})",
        record_id,
        component_id,
        out,
    );

    let id: RecordReference = caller.read(record_id)?;
    let component_id: RecordReference = caller.read(component_id)?;
    let Some(record) = caller.data().as_run()?.records.get(id) else {
        return Ok(1);
    };

    for component in &record.components {
        if component.id == component_id {
            caller.write(out, &(component.bytes.len() as u32))?;
            return Ok(0);
        }
    }

    Ok(1)
}

pub fn get_record_component_get(
    mut caller: Caller<'_, State<'_>>,
    record_id: u32,
    component_id: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("get_record_component_get").entered();
    tracing::trace!(
        "get_record_component_get(record_id={}, component_id={}, out={}, len={})",
        record_id,
        component_id,
        out,
        len,
    );

    let id: RecordReference = caller.read(record_id)?;
    let component_id: RecordReference = caller.read(component_id)?;
    let Some(record) = caller.data().as_run()?.records.get(id) else {
        return Ok(1);
    };

    for component in &record.components {
        if component.id == component_id {
            caller.write_memory(out, &component.bytes)?;
            return Ok(0);
        }
    }

    Ok(1)
}
