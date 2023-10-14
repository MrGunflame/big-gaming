use bytemuck::{Pod, Zeroable};

use super::{Ptr, PtrMut, Usize};
use crate::record::RecordReference;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "host")]
extern "C" {
    pub fn inventory_get(entity_id: u64, slot_id: u64, out: PtrMut<ItemStack>) -> u32;

    pub fn inventory_insert(
        entity_id: u64,
        item_stack: Ptr<ItemStack>,
        slot_id: PtrMut<u64>,
    ) -> u32;

    pub fn inventory_remove(entity_id: u64, id: u64, quantity: u64) -> u32;

    pub fn inventory_clear(entity_id: u64) -> u32;

    pub fn inventory_component_len(
        entity_id: u64,
        slot_id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<Usize>,
    ) -> u32;

    pub fn inventory_component_get(
        entity_id: u64,
        slot_id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<u8>,
        len: Usize,
    ) -> u32;

    pub fn inventory_component_insert(
        entity_id: u64,
        slot_id: u64,
        component_id: Ptr<RecordReference>,
        ptr: Ptr<u8>,
        len: Usize,
    ) -> u32;

    pub fn inventory_component_remove(
        entity_id: u64,
        slot_id: u64,
        component_id: Ptr<RecordReference>,
    ) -> u32;

    pub fn inventory_equip(entity_id: u64, slot_id: u64) -> u32;

    pub fn inventory_unequip(entity_id: u64, slot_id: u64) -> u32;
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_get(
    entity_id: u64,
    slot_id: u64,
    out: PtrMut<ItemStack>,
) -> u32 {
    let _ = (entity_id, slot_id, out);
    panic!("`inventory_get` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_insert(
    entity_id: u64,
    slot_id: u64,
    ptr: Ptr<ItemStack>,
) -> u32 {
    let _ = (entity_id, slot_id, ptr);
    panic!("`inventory_insert` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_remove(entity_id: u64, slot_id: u64) -> u32 {
    let _ = (entity_id, slot_id);
    panic!("`inventory_remove` is not implemented on this target");
}

pub unsafe extern "C" fn inventory_clear(entity_id: u64) -> u32 {
    let _ = entity_id;
    panic!("`inventory_clear` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_component_len(
    entity_id: u64,
    slot_id: u64,
    component_id: Ptr<RecordReference>,
    out: PtrMut<Usize>,
) -> u32 {
    let _ = (entity_id, slot_id, component_id, out);
    panic!("`inventory_component_len` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_component_get(
    entity_id: u64,
    slot_id: u64,
    component_id: Ptr<RecordReference>,
    out: PtrMut<u8>,
    len: Usize,
) -> u32 {
    let _ = (entity_id, slot_id, component_id, out, len);
    panic!("`inventory_component_get` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_component_insert(
    entity_id: u64,
    slot_id: u64,
    component_id: Ptr<RecordReference>,
    ptr: Ptr<u8>,
    len: Usize,
) -> u32 {
    let _ = (entity_id, slot_id, component_id, ptr, len);
    panic!("`inventory_component_insert` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_component_remove(
    entity_id: u64,
    slot_id: u64,
    component_id: Ptr<RecordReference>,
) -> u32 {
    let _ = (entity_id, slot_id, component_id);
    panic!("`inventory_component_remove` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_equip(entity_id: u64, slot_id: u64) -> u32 {
    let _ = (entity_id, slot_id);
    panic!("`inventory_equip` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn inventory_unequip(entity_id: u64, slot_id: u64) -> u32 {
    let _ = (entity_id, slot_id);
    panic!("`inventory_unequip` is not implemented on this target");
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Item {
    pub id: RecordReference,
}
