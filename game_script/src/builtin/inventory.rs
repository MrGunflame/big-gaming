use game_common::components::components::{Component, RecordReference};
use game_common::components::inventory::InventoryId;
use game_common::entity::EntityId;
use wasmtime::{Caller, Result};

use crate::abi::ToAbi;
use crate::instance::State;

use super::CallerExt;

pub fn inventory_get(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    out: u32,
) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let id = InventoryId::from_raw(id);

    let Some(inventory) = caller.data().world.inventories().get(entity_id) else {
        return Ok(1);
    };

    let Some(item) = inventory.get(id) else {
        return Ok(1);
    };

    caller.write(out, &item.to_abi())?;
    Ok(0)
}

pub fn inventory_insert(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    ptr: u32,
) -> Result<u32> {
    todo!()
}

pub fn inventory_remove(mut caller: Caller<'_, State<'_>>, entity_id: u64, id: u64) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let id = InventoryId::from_raw(id);

    let Some(inventory) = caller.data_mut().world.inventories_mut().get_mut(entity_id) else {
        return Ok(1);
    };

    inventory.remove(id);
    Ok(0)
}

pub fn inventory_component_len(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
    out: u32,
) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let id = InventoryId::from_raw(id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(inventory) = caller.data().world.inventories().get(entity_id) else {
       return Ok(1);
    };

    let Some(item) = inventory.get(id) else {
        return Ok(1);
    };

    let Some(component) = item.components.get(component_id) else {
        return Ok(1);
    };

    let len = component.len() as u32;
    caller.write(out, &len)?;
    Ok(0)
}

pub fn inventory_component_get(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let id = InventoryId::from_raw(id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(inventory) = caller.data().world.inventories().get(entity_id) else {
        return Ok(1);
    };

    let Some(item) = inventory.get(id) else {
        return Ok(1);
    };

    let Some(component) = item.components.get(component_id) else {
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

pub fn inventory_component_insert(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
    ptr: u32,
    len: u32,
) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let id = InventoryId::from_raw(id);
    let component_id: RecordReference = caller.read(component_id)?;

    let bytes = caller.read_memory(ptr, len)?.to_owned();

    let Some(inventory) = caller.data_mut().world.inventories_mut().get_mut(entity_id) else {
        return Ok(1);
    };

    let Some(item) = inventory.get_mut(id) else {
        return Ok(1);
    };

    item.components.insert(component_id, Component { bytes });
    Ok(0)
}

pub fn inventory_component_remove(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let id = InventoryId::from_raw(id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(inventory) = caller.data_mut().world.inventories_mut().get_mut(entity_id) else {
        return Ok(1);
    };

    let Some(item) = inventory.get_mut(id) else {
        return Ok(1);
    };

    item.components.remove(component_id);
    Ok(0)
}
