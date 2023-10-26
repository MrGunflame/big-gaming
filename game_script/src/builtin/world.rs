use game_common::components::components::Component;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_tracing::trace_span;
use game_wasm::raw::world::Entity;
use glam::{Quat, Vec3};
use wasmtime::{Caller, Error, Result};

use crate::abi::{FromAbi, ToAbi};
use crate::instance::State;

use super::CallerExt;

const ERROR_NO_ENTITY: u32 = 1;
const ERROR_NO_COMPONENT: u32 = 2;

pub fn world_entity_spawn(mut caller: Caller<'_, State<'_>>, ptr: u32, out: u32) -> Result<u32> {
    let _span = trace_span!("world_entity_spawn").entered();
    tracing::trace!("world_entity_spawn(ptr = {}, out = {})", ptr, out);

    let entity: Entity = caller.read(ptr)?;

    let entity = match entity.from_abi() {
        Ok(entity) => entity,
        Err(err) => return Err(Error::new(err)),
    };

    let id = caller.data_mut().spawn(entity);
    caller.write(out, &id)?;

    Ok(0)
}

pub fn world_entity_get(mut caller: Caller<'_, State<'_>>, id: u64, out: u32) -> Result<u32> {
    let _span = trace_span!("world_entity_get").entered();
    tracing::trace!("world_entity_get(id = {}, out = {})", id, out);

    let entity_id = EntityId::from_raw(id);

    let Some(entity) = caller.data_mut().get(entity_id) else {
        return Ok(ERROR_NO_ENTITY);
    };

    let entity = entity.to_abi();

    caller.write(out, &entity)?;
    Ok(0)
}

pub fn world_entity_despawn(mut caller: Caller<'_, State<'_>>, id: u64) -> Result<u32> {
    let _span = trace_span!("world_entity_despawn").entered();
    tracing::trace!("world_entity_despawn(id = {})", id);

    let id = EntityId::from_raw(id);

    if !caller.data_mut().despawn(id) {
        Ok(ERROR_NO_ENTITY)
    } else {
        Ok(0)
    }
}

pub fn world_entity_component_len(
    mut caller: Caller<'_, State<'_>>,
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

    let Some(entity) = caller.data_mut().get(entity_id) else {
        return Ok(ERROR_NO_ENTITY);
    };

    let Some(component) = caller.data_mut().get_component(entity_id, component_id) else {
        return Ok(ERROR_NO_COMPONENT);
    };

    let len = component.len() as u32;

    caller.write(out, &len)?;
    Ok(0)
}

pub fn world_entity_component_get(
    mut caller: Caller<'_, State<'_>>,
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

    let Some(entity) = caller.data_mut().get(entity_id) else {
        return Ok(ERROR_NO_ENTITY);
    };

    let Some(component) = caller.data_mut().get_component(entity_id, component_id) else {
        return Ok(ERROR_NO_COMPONENT);
    };

    let mut bytes = component.as_bytes();
    if bytes.len() as u32 > len {
        bytes = &bytes[..len as usize];
    }

    // FIXME: We shouldn't have to clone here.
    let bytes = bytes.to_owned();

    caller.write_memory(out, &bytes)?;
    Ok(0)
}

pub fn world_entity_component_insert(
    mut caller: Caller<'_, State<'_>>,
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

    let Some(mut entity) = caller.data_mut().get(entity_id) else {
        return Ok(ERROR_NO_ENTITY);
    };

    caller
        .data_mut()
        .insert_component(entity_id, component_id, Component { bytes });

    Ok(0)
}

pub fn world_entity_component_remove(
    mut caller: Caller<'_, State<'_>>,
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

    let Some(mut entity) = caller.data_mut().get(entity_id) else {
        return Ok(ERROR_NO_ENTITY);
    };

    if !caller.data_mut().remove_component(entity_id, component_id) {
        Ok(ERROR_NO_COMPONENT)
    } else {
        Ok(0)
    }
}

pub fn world_entity_set_translation(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_set_translation").entered();
    tracing::trace!(
        "world_entity_set_translation(entity_id = {}, x = {}, y = {}, z = {})",
        entity_id,
        x,
        y,
        z
    );

    let entity_id = EntityId::from_raw(entity_id);
    let translation = Vec3::new(x, y, z);

    if !caller.data_mut().set_translation(entity_id, translation) {
        return Ok(ERROR_NO_ENTITY);
    }

    Ok(0)
}

pub fn world_entity_set_rotation(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
) -> Result<u32> {
    let _span = trace_span!("world_entity_set_rotation").entered();
    tracing::trace!(
        "world_entity_set_rotation(entity_id = {}, x = {}, y = {}, z = {}, w = {}",
        entity_id,
        x,
        y,
        z,
        w
    );

    let entity_id = EntityId::from_raw(entity_id);
    let rotation = Quat::from_xyzw(x, y, z, w);
    assert!(rotation.is_normalized());

    if !caller.data_mut().set_rotation(entity_id, rotation) {
        return Ok(ERROR_NO_ENTITY);
    };

    Ok(0)
}
