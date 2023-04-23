use wasmtime::{Caller, Result};

use crate::instance::State;

pub fn inventory_get(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    out: u32,
) -> Result<u32> {
    todo!();
}

pub fn inventory_insert(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    ptr: u32,
) -> Result<u32> {
    todo!()
}

pub fn inventory_remove(caller: Caller<'_, State<'_>>, entity_id: u64, id: u64) -> Result<u32> {
    todo!()
}

pub fn inventory_component_len(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
    out: u32,
) -> Result<u32> {
    todo!()
}

pub fn inventory_component_get(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
    out: u32,
    len: u32,
) -> Result<u32> {
    todo!()
}

pub fn inventory_component_insert(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
    ptr: u32,
    len: u32,
) -> Result<u32> {
    todo!()
}

pub fn inventory_component_remove(
    caller: Caller<'_, State<'_>>,
    entity_id: u64,
    id: u64,
    component_id: u32,
) -> Result<u32> {
    todo!()
}
