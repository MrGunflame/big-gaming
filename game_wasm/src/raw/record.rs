use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use crate::record::ModuleId;
use crate::world::RecordReference;

#[guest_only]
pub fn record_list_count(filter: *const RawRecordFilter, out: *mut usize) -> u32;

#[guest_only]
pub fn record_list_copy(
    filter: *const RawRecordFilter,
    out: *mut RecordReference,
    len: usize,
) -> u32;

/// Returns the data length of the record.
#[guest_only]
pub fn record_data_len(id: *const RecordReference, out: *mut usize) -> u32;

/// Copies up to `len`-bytes into `dst`.
#[guest_only]
pub fn record_data_copy(id: *const RecordReference, dst: *mut u8, len: usize) -> u32;

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct RawRecordFilter {
    pub filter_module: u8,
    pub filter_kind: u8,
    pub _pad0: u16,
    pub module: ModuleId,
    pub kind: RecordReference,
}
