use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use crate::world::RecordReference;

use super::{Ptr, PtrMut, Usize};

#[guest_only]
pub fn get_record(id: Ptr<RecordReference>, out: PtrMut<Record>) -> u32;

#[guest_only]
pub fn get_record_len_component(id: Ptr<RecordReference>, out: PtrMut<Usize>) -> u32;

#[guest_only]
pub fn get_record_component_keys(
    id: Ptr<RecordReference>,
    out: PtrMut<RecordReference>,
    len: Usize,
) -> u32;

#[guest_only]
pub fn get_record_component_len(
    id: Ptr<RecordReference>,
    component_id: Ptr<RecordReference>,
    out: PtrMut<Usize>,
) -> u32;

#[guest_only]
pub fn get_record_component_get(
    id: Ptr<RecordReference>,
    component_id: Ptr<RecordReference>,
    ptr: PtrMut<u8>,
    len: Usize,
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
