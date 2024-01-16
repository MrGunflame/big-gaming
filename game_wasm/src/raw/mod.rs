pub mod components;
pub mod inventory;
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

#[guest_only]
pub fn log(level: u32, ptr: *const u8, len: usize);

#[guest_only]
pub fn player_lookup(entity_id: u64, player_id: *mut u64) -> u32;

#[guest_only]
pub fn player_set_active(player_id: u64, entity_id: u64) -> u32;

#[guest_only]
pub fn action_data_buffer_len() -> usize;

#[guest_only]
pub fn action_data_buffer_get(ptr: *mut u8);

#[guest_only]
pub fn event_dispatch(
    id: *const RecordReference,
    rx: *const EventReceiver,
    ptr: *const u8,
    len: usize,
);

#[guest_only]
pub fn register_system(query: *const Query, fn_ptr: *const unsafe fn(c_void));

#[guest_only]
pub fn register_event_handler(id: *const RecordReference, ptr: *const unsafe fn(c_void));

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Query {
    pub components_ptr: *const RecordReference,
    pub components_len: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct EventReceiver {
    /// core::ptr::null() indicates all entities.
    pub entities_ptr: *const u64,
    pub entities_len: usize,
}
