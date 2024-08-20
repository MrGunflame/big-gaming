pub mod components;
pub mod physics;
pub mod process;
pub mod record;
pub mod world;

use core::ffi::c_void;

use game_macros::guest_only;

use crate::record::RecordReference;

pub const RESULT_OK: u32 = 0;
pub const RESULT_NO_ENTITY: u32 = 1;
pub const RESULT_NO_COMPONENT: u32 = 2;
pub const RESULT_NO_INVENTORY_SLOT: u32 = 3;
pub const RESULT_NO_RECORD: u32 = 1;

#[guest_only]
pub fn log(level: u32, ptr: *const u8, len: usize);

#[guest_only]
pub fn player_lookup(entity_id: u64, player_id: *mut u64) -> u32;

#[guest_only]
pub fn player_set_active(player_id: u64, entity_id: u64) -> u32;

#[guest_only]
pub fn host_buffer_len(index: u32) -> usize;

#[guest_only]
pub fn host_buffer_get(index: u32, ptr: *mut u8);

#[guest_only]
pub fn event_dispatch(
    id: *const RecordReference,
    data_ptr: *const u8,
    data_len: usize,
    fields_ptr: *const u8,
    fields_len: usize,
);

#[guest_only]
pub fn register_system(query: *const Query, fn_ptr: *const unsafe fn(u64, c_void));

#[guest_only]
pub fn register_event_handler(id: *const RecordReference, ptr: *const unsafe fn(u64, c_void));

#[guest_only]
pub fn register_action_handler(id: *const RecordReference, ptr: *const unsafe fn(u64, c_void));

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Query {
    pub components_ptr: *const RecordReference,
    pub components_len: usize,
}

#[guest_only]
pub fn prefab_spawn(id: *const RecordReference, out: *mut u64) -> u32;

#[guest_only]
pub fn create_resource(ptr: *const u8, len: usize) -> u64;

#[guest_only]
pub fn bind_resource(id: u64);
