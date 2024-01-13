use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use crate::world::RecordReference;

#[guest_only]
pub fn get_record(id: *const RecordReference, out: *mut Record) -> u32;

#[guest_only]
pub fn get_record_len_component(id: *const RecordReference, out: *mut usize) -> u32;

#[guest_only]
pub fn get_record_component_keys(
    id: *const RecordReference,
    out: *mut RecordReference,
    len: usize,
) -> u32;

#[guest_only]
pub fn get_record_component_len(
    id: *const RecordReference,
    component_id: *const RecordReference,
    out: *mut usize,
) -> u32;

#[guest_only]
pub fn get_record_component_get(
    id: *const RecordReference,
    component_id: *const RecordReference,
    ptr: *mut u8,
    len: usize,
) -> u32;

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Record {
    pub kind: RecordKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordKind(u32);

impl RecordKind {
    pub const ITEM: Self = Self(1);
    pub const OBJECT: Self = Self(2);
    pub const RACE: Self = Self(3);
}
