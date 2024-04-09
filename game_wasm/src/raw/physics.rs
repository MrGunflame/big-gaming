use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use crate::entity::EntityId;

#[guest_only]
pub fn physics_cast_ray(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    max_toi: f32,
    filter_ptr: *const QueryFilter,
    out: *mut CastRayResult,
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
    shape_type: u32,
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

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct QueryFilter {
    pub exclude_entities_ptr: *const EntityId,
    pub exclude_entities_len: usize,
}

#[repr(C)]
pub union Shape {
    pub cuboid: Cuboid,
    pub ball: Ball,
    pub capsule: Capsule,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Ball {
    pub radius: f32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Capsule {
    pub axis: u32,
    pub half_height: f32,
    pub radius: f32,
}

pub const SHAPE_TYPE_CUBOID: u32 = 1;
pub const SHAPE_TYPE_BALL: u32 = 2;
pub const SHAPE_TYPE_CAPSULE: u32 = 3;
