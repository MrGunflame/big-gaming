use game_common::components::components::Component;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_wasm::raw::world::Entity;
use wasmtime::{Caller, Error, Result};

use crate::abi::{FromAbi, ToAbi};
use crate::instance::State;

use super::CallerExt;

pub fn world_entity_spawn(mut caller: Caller<'_, State<'_>>, ptr: u32, out: u32) -> Result<u32> {
    tracing::trace!("world_entity_spawn(ptr = {}, out = {})", ptr, out);

    let entity: Entity = caller.read(ptr)?;

    let entity = match entity.from_abi() {
        Ok(entity) => entity,
        Err(err) => return Err(Error::new(err)),
    };

    let id = caller.data_mut().world.spawn(entity);
    caller.write(out, &id)?;

    Ok(0)
}

pub fn world_entity_get(mut caller: Caller<'_, State<'_>>, id: u64, out: u32) -> Result<u32> {
    tracing::trace!("world_entity_get(id = {}, out = {})", id, out);

    let Some(entity) = caller.data_mut().world.get(EntityId::from_raw(id)) else {
       return Ok(1);
    };

    let entity = entity.to_abi();

    caller.write(out, &entity)?;
    Ok(0)
}

pub fn world_entity_despawn(mut caller: Caller<'_, State<'_>>, id: u64) -> Result<u32> {
    tracing::trace!("world_entity_despawn(id = {})", id);

    let id = EntityId::from_raw(id);

    caller.data_mut().world.despawn(id);
    Ok(0)
}

pub fn world_entity_component_len(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    component_id: u32,
    out: u32,
) -> Result<u32> {
    tracing::trace!(
        "world_entity_component_len(entity_id = {}, component_id = {}, out = {})",
        entity_id,
        component_id,
        out
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(entity) = caller.data().world.get(entity_id) else {
        return Ok(1);
    };

    let Some(component) = entity.components.get(component_id) else {
        return Ok(1);
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
    tracing::trace!(
        "world_entity_component_get(entity_id = {}, component_id = {}, out = {}, len = {})",
        entity_id,
        component_id,
        out,
        len,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(entity) = caller.data().world.get(entity_id) else {
        return Ok(1);
    };

    let Some(component) = entity.components.get(component_id) else {
        return Ok(1);
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

    let Some(mut entity) = caller.data_mut().world.get_mut(entity_id) else {
        return Ok(1);
    };

    entity.components.insert(component_id, Component { bytes });
    Ok(0)
}

pub fn world_entity_component_remove(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    component_id: u32,
) -> Result<u32> {
    tracing::trace!(
        "world_entity_component_remove(entity_id = {}, component_id = {})",
        entity_id,
        component_id,
    );

    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(mut entity) = caller.data_mut().world.get_mut(entity_id) else {
        return Ok(1);
    };

    entity.components.remove(component_id);
    Ok(0)
}
