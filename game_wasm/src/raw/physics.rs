use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use super::{Ptr, PtrMut, Usize};

#[guest_only]
pub fn physics_cast_ray(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    max_toi: f32,
    filter_ptr: Ptr<QueryFilter>,
    out: PtrMut<CastRayResult>,
) -> u32;

#[guest_only]
pub fn physics_cast_shape(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    rotation_w: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    shape: *const Shape,
    max_toi: f32,
    filter: *const QueryFilter,
    out: *mut CastRayResult,
) -> u32;

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct CastRayResult {
    pub entity_id: u64,
    pub toi: f32,
    pub _pad0: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct QueryFilter {
    // FIXME: Maybe change to `Ptr<EntityId>`.
    pub exclude_entities_ptr: Usize,
    pub exclude_entities_len: Usize,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Shape {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}
