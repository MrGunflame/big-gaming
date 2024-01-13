use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use crate::record::RecordReference;

#[guest_only]
pub fn inventory_list(entity_id: u64, out: *mut u64, len: u32) -> u32;

#[guest_only]
pub fn inventory_len(entity_id: u64, out: *mut u32) -> u32;

#[guest_only]
pub fn inventory_get(entity_id: u64, slot_id: u64, out: *mut ItemStack) -> u32;

#[guest_only]
pub fn inventory_insert(entity_id: u64, item_stack: *const ItemStack, slot_id: *mut u64) -> u32;

#[guest_only]
pub fn inventory_remove(entity_id: u64, id: u64, quantity: u64) -> u32;

#[guest_only]
pub fn inventory_clear(entity_id: u64) -> u32;

#[guest_only]
pub fn inventory_component_len(
    entity_id: u64,
    slot_id: u64,
    component_id: *const RecordReference,
    out: *mut usize,
) -> u32;

#[guest_only]
pub fn inventory_component_get(
    entity_id: u64,
    slot_id: u64,
    component_id: *const RecordReference,
    out: *mut u8,
    len: usize,
) -> u32;

#[guest_only]
pub fn inventory_component_insert(
    entity_id: u64,
    slot_id: u64,
    component_id: *const RecordReference,
    ptr: *const u8,
    len: usize,
) -> u32;

#[guest_only]
pub fn inventory_component_remove(
    entity_id: u64,
    slot_id: u64,
    component_id: *const RecordReference,
) -> u32;

#[guest_only]
pub fn inventory_equip(entity_id: u64, slot_id: u64) -> u32;

#[guest_only]
pub fn inventory_unequip(entity_id: u64, slot_id: u64) -> u32;

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
    // Note that `equipped` and `hidden` flags are currently only for reads.
    pub equipped: u8,
    pub hdden: u8,
    pub _pad0: u16,
}
