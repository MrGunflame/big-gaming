use game_macros::guest_only;

use crate::world::RecordReference;

/// Returns the data length of the record.
#[guest_only]
pub fn record_data_len(id: *const RecordReference, out: *mut usize) -> u32;

/// Copies up to `len`-bytes into `dst`.
#[guest_only]
pub fn record_data_copy(id: *const RecordReference, dst: *mut u8, len: usize) -> u32;
