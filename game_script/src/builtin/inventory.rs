use game_common::components::components::RawComponent;
use game_common::components::inventory::InventorySlotId;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_tracing::trace_span;
use game_wasm::raw::inventory::ItemStack;
use game_wasm::raw::RESULT_OK;
use wasmtime::{Caller, Error, Result};

use crate::abi::{FromAbi, ToAbi};
use crate::instance::State;

use super::CallerExt;

pub fn inventory_get(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_get").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);

    let Some(stack) = caller
        .data_mut()
        .as_run_mut()?
        .inventory_get(entity_id, slot_id)
    else {
        return Ok(1);
    };

    caller.write(out, &stack.to_abi())?;
    Ok(RESULT_OK)
}

pub fn inventory_len(mut caller: Caller<'_, State>, entity_id: u64, out: u32) -> Result<u32> {
    let _span = trace_span!("inventory_len").entered();

    let entity_id = EntityId::from_raw(entity_id);

    let Some(inventory) = caller.data_mut().as_run_mut()?.inventory(entity_id) else {
        return Ok(1);
    };

    caller.write(out, &(inventory.len() as u32))?;
    Ok(RESULT_OK)
}

pub fn inventory_list(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    out: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_list").entered();

    let entity_id = EntityId::from_raw(entity_id);

    let Some(inventory) = caller.data_mut().as_run_mut()?.inventory(entity_id) else {
        return Ok(1);
    };

    // Write at most len elements.
    for ((id, _), index) in inventory.iter().zip(0..len) {
        let ptr = out + (index * std::mem::size_of::<ItemStack>() as u32);
        caller.write(ptr, &id.into_raw())?;
    }

    Ok(RESULT_OK)
}

pub fn inventory_insert(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    item_stack_ptr: u32,
    slot_id_ptr: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_insert").entered();

    let entity_id = EntityId::from_raw(entity_id);

    let stack = match caller.read::<ItemStack>(item_stack_ptr)?.from_abi() {
        Ok(stack) => stack,
        Err(err) => return Err(Error::new(err)),
    };

    let id = caller
        .data_mut()
        .as_run_mut()?
        .inventory_insert(entity_id, stack);

    caller.write(slot_id_ptr, &id)?;
    Ok(RESULT_OK)
}

pub fn inventory_remove(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
    quantity: u64,
) -> Result<u32> {
    let _span = trace_span!("inventory_remove").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);

    if let Err(err) = caller
        .data_mut()
        .as_run_mut()?
        .inventory_remove(entity_id, slot_id, quantity)
    {
        return Ok(err.to_u32());
    }

    Ok(RESULT_OK)
}

pub fn inventory_component_len(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
    component_id: u32,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_component_len").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(component) =
        caller
            .data_mut()
            .as_run_mut()?
            .inventory_component_get(entity_id, slot_id, component_id)
    else {
        return Ok(1);
    };

    let len = component.len() as u32;
    caller.write(out, &len)?;
    Ok(RESULT_OK)
}

pub fn inventory_component_get(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
    component_id: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_component_get").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let Some(component) =
        caller
            .data_mut()
            .as_run_mut()?
            .inventory_component_get(entity_id, slot_id, component_id)
    else {
        return Ok(1);
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

pub fn inventory_component_insert(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
    component_id: u32,
    ptr: u32,
    len: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_component_insert").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);
    let component_id: RecordReference = caller.read(component_id)?;

    let bytes = caller.read_memory(ptr, len)?.to_owned();

    if let Err(err) = caller.data_mut().as_run_mut()?.inventory_component_insert(
        entity_id,
        slot_id,
        component_id,
        RawComponent::new(bytes, vec![]),
    ) {
        return Ok(err.to_u32());
    };

    Ok(RESULT_OK)
}

pub fn inventory_component_remove(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
    component_id: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_component_remove").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);
    let component_id: RecordReference = caller.read(component_id)?;

    if let Err(err) =
        caller
            .data_mut()
            .as_run_mut()?
            .inventory_component_remove(entity_id, slot_id, component_id)
    {
        return Ok(err.to_u32());
    };

    Ok(RESULT_OK)
}

pub fn inventory_equip(mut caller: Caller<'_, State>, entity_id: u64, slot_id: u64) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);

    if let Err(err) = caller
        .data_mut()
        .as_run_mut()?
        .inventory_set_equipped(entity_id, slot_id, true)
    {
        return Ok(err.to_u32());
    }

    Ok(RESULT_OK)
}

// FIXME: It probably does make more sense to merge this into `inventory_equip` with
// a bool param.
pub fn inventory_unequip(
    mut caller: Caller<'_, State>,
    entity_id: u64,
    slot_id: u64,
) -> Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);

    if let Err(err) = caller
        .data_mut()
        .as_run_mut()?
        .inventory_set_equipped(entity_id, slot_id, false)
    {
        return Ok(err.to_u32());
    }

    Ok(RESULT_OK)
}

pub fn inventory_clear(mut caller: Caller<'_, State>, entity_id: u64) -> Result<u32> {
    let _span = trace_span!("inventory_clear").entered();

    let entity_id = EntityId::from_raw(entity_id);

    if let Err(err) = caller.data_mut().as_run_mut()?.inventory_clear(entity_id) {
        return Ok(err.to_u32());
    };

    Ok(RESULT_OK)
}
