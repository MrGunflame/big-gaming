use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use super::PtrMut;

#[guest_only]
pub fn health_get(entity_id: u64, out: PtrMut<Health>) -> u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct Health(pub u32);
