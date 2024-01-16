use game_common::components::components::RawComponent;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_tracing::trace_span;
use game_wasm::raw::{RESULT_NO_COMPONENT, RESULT_NO_ENTITY, RESULT_OK};
use wasmtime::{Caller, Result};

use crate::instance::State;

use super::CallerExt;

pub fn world_entity_spawn(mut caller: Caller<'_, State>, out: u32) -> Result<u32> {
    let _span = trace_span!("world_entity_spawn").entered();
    tracing::trace!("world_entity_spawn(out = {})", out);

    let id = caller.data_mut().as_run_mut()?.spawn();
    caller.write(out, &id)?;

    Ok(RESULT_OK)
}

pub fn world_entity_despawn(mut caller: Caller<'_, State>, id: u64) -> Result<u32> {
    let _span = trace_span!("world_entity_despawn").entered();
    tracing::trace!("world_entity_despawn(id = {})", id);

    let id = EntityId::from_raw(id);

    if !caller.data_mut().as_run_mut()?.despawn(id) {
        Ok(RESULT_NO_ENTITY)
    } else {
        Ok(RESULT_OK)
    }
}

pub fn world_entity_component_len(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_len").entered();
    tracing::trace!(
        "world_entity_component_len(entity_id = {}, component_id = {}, out = {})",
        entity_id,
        component_id,
        out
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(component) = caller
        .data_mut()
        .as_run_mut()?
        .get_component(entity_id, component_id)
    else {
        return Ok(RESULT_NO_COMPONENT);
    };

    let len = component.len() as u32;

    caller.write(out, &len)?;
    Ok(RESULT_OK)
}

pub fn world_entity_component_get(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_get").entered();
    tracing::trace!(
        "world_entity_component_get(entity_id = {}, component_id = {}, out = {}, len = {})",
        entity_id,
        component_id,
        out,
        len,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(component) = caller
        .data_mut()
        .as_run_mut()?
        .get_component(entity_id, component_id)
    else {
        return Ok(RESULT_NO_COMPONENT);
    };

    let mut bytes = component.as_bytes();
    if bytes.len() as u32 > len {
        bytes = &bytes[..len as usize];
    }

    // FIXME: We shouldn't have to clone here.
    let bytes = bytes.to_owned();

    caller.write_memory(out, &bytes)?;
    Ok(RESULT_OK)
}

pub fn world_entity_component_insert(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
    ptr: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_insert").entered();
    tracing::trace!(
        "world_entity_component_insert(entity_id = {}, component_id = {}, ptr = {}, len = {})",
        entity_id,
        component_id,
        ptr,
        len,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;
    let bytes = caller.read_memory(ptr, len)?.to_owned();

    caller.data_mut().as_run_mut()?.insert_component(
        entity_id,
        component_id,
        RawComponent::new(bytes),
    );

    Ok(RESULT_OK)
}

pub fn world_entity_component_remove(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_remove").entered();
    tracing::trace!(
        "world_entity_component_remove(entity_id = {}, component_id = {})",
        entity_id,
        component_id,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    if !caller
        .data_mut()
        .as_run_mut()?
        .remove_component(entity_id, component_id)
    {
        Ok(RESULT_NO_COMPONENT)
    } else {
        Ok(RESULT_OK)
    }
}
