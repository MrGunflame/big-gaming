use game_common::components::components::Component;
use game_common::components::inventory::InventorySlotId;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_tracing::trace_span;
use game_wasm::raw::inventory::ItemStack;
use wasmtime::{Caller, Error, Result};

use crate::abi::{FromAbi, ToAbi};
use crate::instance::State;

use super::CallerExt;

pub fn inventory_get(
    mut caller: Caller<'_, State<'_, '_>>,
    entity_id: u64,
    slot_id: u64,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_get").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);

    let Some(stack) = caller.data_mut().inventory_get(entity_id, slot_id) else {
        return Ok(1);
    };

    caller.write(out, &stack.to_abi())?;
    Ok(0)
}

pub fn inventory_insert(
    mut caller: Caller<'_, State<'_, '_>>,
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

    let id = caller.data_mut().inventory_insert(entity_id, stack);

    caller.write(slot_id_ptr, &id)?;
    Ok(0)
}

pub fn inventory_remove(
    mut caller: Caller<'_, State<'_, '_>>,
    entity_id: u64,
    slot_id: u64,
    quantity: u64,
) -> Result<u32> {
    let _span = trace_span!("inventory_remove").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);

    if !caller
        .data_mut()
        .inventory_remove(entity_id, slot_id, quantity)
    {
        return Ok(1);
    }

    Ok(0)
}

pub fn inventory_component_len(
    mut caller: Caller<'_, State<'_, '_>>,
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
            .inventory_component_get(entity_id, slot_id, component_id)
    else {
        return Ok(1);
    };

    let len = component.len() as u32;
    caller.write(out, &len)?;
    Ok(0)
}

pub fn inventory_component_get(
    mut caller: Caller<'_, State<'_, '_>>,
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
    Ok(0)
}

pub fn inventory_component_insert(
    mut caller: Caller<'_, State<'_, '_>>,
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

    if !caller.data_mut().inventory_component_insert(
        entity_id,
        slot_id,
        component_id,
        Component { bytes },
    ) {
        return Ok(1);
    };

    Ok(0)
}

pub fn inventory_component_remove(
    mut caller: Caller<'_, State<'_, '_>>,
    entity_id: u64,
    slot_id: u64,
    component_id: u32,
) -> Result<u32> {
    let _span = trace_span!("inventory_component_remove").entered();

    let entity_id = EntityId::from_raw(entity_id);
    let slot_id = InventorySlotId::from_raw(slot_id);
    let component_id: RecordReference = caller.read(component_id)?;

    if !caller
        .data_mut()
        .inventory_component_remove(entity_id, slot_id, component_id)
    {
        return Ok(1);
    };

    Ok(0)
}

pub fn inventory_equip(
    mut caller: Caller<'_, State<'_, '_>>,
    entity_id: u64,
    slot_id: u64,
) -> Result<u32> {
    // let entity_id = EntityId::from_raw(entity_id);
    // let id = InventorySlotId::from_raw(slot_id);

    // todo!();
    // let inventories = caller.data_mut().world.inventories_mut();
    // let Some(inventory) = inventories.get_mut(entity_id) else {
    //     return Ok(1);
    // };

    // let Some(stack) = inventory.get_mut(id) else {
    //     return Ok(1);
    // };

    // // ItemMut drop does the rest.
    // stack.item.equipped = true;
    Ok(0)
}

pub fn inventory_unequip(
    mut caller: Caller<'_, State<'_, '_>>,
    entity_id: u64,
    slot_id: u64,
) -> Result<u32> {
    // let entity_id = EntityId::from_raw(entity_id);
    // let id = InventorySlotId::from_raw(slot_id);

    // let inventories = caller.data_mut().world.inventories_mut();
    // let Some(inventory) = inventories.get_mut(entity_id) else {
    //     return Ok(1);
    // };

    // let Some(stack) = inventory.get_mut(id) else {
    //     return Ok(1);
    // };

    // // ItemMut drop does the rest.
    // stack.item.equipped = false;
    Ok(0)
}

pub fn inventory_clear(mut caller: Caller<'_, State<'_, '_>>, entity_id: u64) -> Result<u32> {
    let _span = trace_span!("inventory_clear").entered();

    let entity_id = EntityId::from_raw(entity_id);

    if !caller.data_mut().inventory_clear(entity_id) {
        return Ok(1);
    };

    Ok(0)
}
