pub mod components;
pub mod inventory;
pub mod physics;
pub mod process;
pub mod record;
pub mod world;

use game_macros::guest_only;

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
