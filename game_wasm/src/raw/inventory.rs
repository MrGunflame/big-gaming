use bytemuck::{Pod, Zeroable};

use crate::world::RecordReference;

use super::{Ptr, PtrMut, Usize};

#[link(wasm_import_module = "host")]
extern "C" {
    pub fn inventory_get(entity_id: u64, id: u64, out: PtrMut<Item>) -> u32;

    pub fn inventory_insert(entity_id: u64, id: i64, ptr: Ptr<Item>) -> u32;

    pub fn inventory_remove(entity_id: u64, id: u64) -> u32;

    pub fn inventory_component_len(
        entity_id: u64,
        id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<Usize>,
    ) -> u32;

    pub fn inventory_component_get(
        entity_id: u64,
        id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<u8>,
        len: Usize,
    ) -> u32;

    pub fn inventory_component_insert(
        entity_id: u64,
        id: u64,
        component_id: Ptr<RecordReference>,
        ptr: Ptr<u8>,
        len: Usize,
    ) -> u32;

    pub fn inventory_component_remove(
        entity_id: u64,
        id: u64,
        component_id: Ptr<RecordReference>,
    ) -> u32;

}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Item {
    pub id: RecordReference,
}
