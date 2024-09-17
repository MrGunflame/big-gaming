use std::sync::Arc;

use game_common::components::components::RawComponent;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_prefab::{Prefab, Spawner};
use game_tracing::trace_span;
use game_wasm::encoding::{decode_fields, encode_fields, Field};
use game_wasm::raw::{RESULT_NO_ENTITY, RESULT_NO_RECORD, RESULT_OK};
use game_wasm::resource::RuntimeResourceId;
use wasmtime::{Caller, Result};

use crate::builtin::CallerExt;
use crate::instance::{RunState, State};

use super::AsMemory;

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

    match caller.data_mut().as_run_mut()?.despawn(id) {
        Ok(()) => Ok(RESULT_OK),
        Err(err) => Ok(err.to_u32()),
    }
}

pub fn world_entity_component_len(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
    data_len_out: u32,
    fields_len_out: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_len").entered();
    tracing::trace!(
        "world_entity_component_len(entity_id = {}, component_id = {}, data_len_out = {}, fields_len_out = {})",
        entity_id,
        component_id,
        data_len_out,
        fields_len_out,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let component = match caller
        .data_mut()
        .as_run_mut()?
        .get_component(entity_id, component_id)
    {
        Ok(component) => component,
        Err(err) => return Ok(err.to_u32()),
    };

    let data_len = component.as_bytes().len() as u32;
    let fields_len = component.fields().len() * Field::ENCODED_SIZE;

    caller.write(data_len_out, &data_len)?;
    caller.write(fields_len_out, &fields_len)?;

    Ok(RESULT_OK)
}

pub fn world_entity_component_get(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
    data_out: u32,
    fields_out: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_get").entered();
    tracing::trace!(
        "world_entity_component_get(entity_id = {}, component_id = {}, data_out = {}, fields_out = {})",
        entity_id,
        component_id,
        data_out,
        fields_out,
    );

    let (mut memory, data) = caller.split()?;

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = memory.read(component_id)?;

    let component = match data.as_run_mut()?.get_component(entity_id, component_id) {
        Ok(component) => component,
        Err(err) => return Ok(err.to_u32()),
    };

    // Note that a null pointer indicates that the guest does not request that
    // information and we should skip writing to it.
    if data_out != 0 {
        memory.write_memory(data_out, component.as_bytes())?;
    }

    if fields_out != 0 {
        let fields = component.fields();
        let fields = encode_fields(fields);
        memory.write_memory(fields_out, &fields)?;
    }

    Ok(RESULT_OK)
}

pub fn world_entity_component_insert(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    component_id: u32,
    data_ptr: u32,
    data_len: u32,
    fields_ptr: u32,
    fields_len: u32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_component_insert").entered();
    tracing::trace!(
        "world_entity_component_insert(entity_id = {}, component_id = {}, data_ptr = {}, data_len = {}, fields_ptr = {}, fields_len = {})",
        entity_id,
        component_id,
        data_ptr,
        data_len,
        fields_ptr,
        fields_len,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let data = caller.read_memory(data_ptr, data_len)?.to_vec();
    let fields = caller.read_memory(fields_ptr, fields_len)?;
    let fields = decode_fields(fields);

    let component = RawComponent::new(data, fields);
    match caller
        .data_mut()
        .as_run_mut()?
        .insert_component(entity_id, component_id, component)
    {
        Ok(()) => Ok(RESULT_OK),
        Err(err) => Ok(err.to_u32()),
    }
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

    match caller
        .data_mut()
        .as_run_mut()?
        .remove_component(entity_id, component_id)
    {
        Ok(()) => Ok(RESULT_OK),
        Err(err) => Ok(err.to_u32()),
    }
}

pub fn prefab_spawn(mut caller: Caller<'_, State>, id: u32, out: u32) -> Result<u32> {
    let _span = trace_span!("prefab_spawn").entered();

    let id: RecordReference = caller.read(id)?;
    let data = caller.data_mut().as_run_mut()?;

    let Some(record) = data.records().get(id) else {
        return Ok(RESULT_NO_RECORD);
    };

    // FIXME: We shouldn't have to decode here. The record should
    // be decoded when the module is loaded, then we can instantiate
    // it without having it fail here.
    let prefab = match Prefab::from_bytes(&record.data) {
        Ok(prefab) => prefab,
        Err(err) => {
            tracing::error!("failed to decode prefab record: {}", err);
            return Ok(RESULT_NO_RECORD);
        }
    };

    let root = prefab.instantiate(PrefabSpawner { state: data });
    caller.write(out, &root)?;

    Ok(RESULT_OK)
}

struct PrefabSpawner<'a> {
    state: &'a mut RunState,
}

impl<'a> Spawner for PrefabSpawner<'a> {
    fn spawn(&mut self) -> EntityId {
        self.state.spawn()
    }

    fn insert(&mut self, entity: EntityId, component_id: RecordReference, component: RawComponent) {
        // `insert_component` returns an error if the `entity` does not exist,
        // but `Prefab::instantiate` guarantees to always spawn the entity
        // before inserting components, making this error unreachable.
        self.state
            .insert_component(entity, component_id, component)
            .unwrap();
    }
}

/// ```no_run
/// # extern "C" {
/// fn resource_create_runtime(ptr: *const u8, len: usize, out: *mut u64) -> u32;
/// # }
/// ```
///
pub fn resource_create_runtime(
    mut caller: Caller<'_, State>,
    ptr: u32,
    len: u32,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("resource_create_runtime").entered();

    let data = Arc::from(caller.read_memory(ptr, len)?.to_vec());
    let state = caller.data_mut().as_run_mut()?;
    let id = state.insert_resource(data);
    caller.write(out, &id.to_bits())?;
    Ok(RESULT_OK)
}

/// ```no_run
/// # extern "C" {
/// fn resource_destroy_runtime(id: u64) -> u32;
/// # }
/// ```
pub fn resource_destroy_runtime(mut caller: Caller<'_, State>, id: u64) -> Result<u32> {
    let _span = trace_span!("resource_destroy_runtime").entered();

    let state = caller.data_mut().as_run_mut()?;
    if state.destroy_resource(RuntimeResourceId::from_bits(id)) {
        Ok(RESULT_OK)
    } else {
        Ok(RESULT_NO_ENTITY)
    }
}

/// ```no_run
/// # extern "C" {
/// fn resource_len_runtime(id: u64, out: *mut usize) -> u32;
/// # }
/// ```
pub fn resource_len_runtime(mut caller: Caller<'_, State>, id: u64, out: u32) -> Result<u32> {
    let _span = trace_span!("resource_len_runtime").entered();

    let state = caller.data_mut().as_run_mut()?;
    let Some(data) = state.get_resource_runtime(RuntimeResourceId::from_bits(id)) else {
        return Ok(RESULT_NO_ENTITY);
    };

    let len = data.len() as u32;
    caller.write(out, &len)?;
    Ok(RESULT_OK)
}

/// ```no_run
/// # extern "C" {
/// fn resource_get_runtime(id: u64, ptr: *mut u8) -> u32;
/// # }
/// ```
pub fn resource_get_runtime(mut caller: Caller<'_, State>, id: u64, ptr: u32) -> Result<u32> {
    let _span = trace_span!("resource_get_runtime").entered();

    let (mut memory, data) = caller.split()?;

    let state = data.as_run_mut()?;
    let Some(data) = state.get_resource_runtime(RuntimeResourceId::from_bits(id)) else {
        return Ok(RESULT_NO_ENTITY);
    };

    memory.write_memory(ptr, data)?;
    Ok(RESULT_OK)
}

/// ```no_run
/// # extern "C" {
/// fn resource_update_runtime(id: u64, ptr: *mut u8, len: usize) -> u32;
/// # }
pub fn resource_update_runtime(
    mut caller: Caller<'_, State>,
    id: u64,
    ptr: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("resource_update_runtime").entered();

    let id = RuntimeResourceId::from_bits(id);
    let data = Arc::from(caller.read_memory(ptr, len)?.to_vec());

    let state = caller.data_mut().as_run_mut()?;
    if !state.update_resource(id, data) {
        Ok(RESULT_NO_ENTITY)
    } else {
        Ok(RESULT_OK)
    }
}
