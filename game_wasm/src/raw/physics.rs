use bytemuck::{Pod, Zeroable};

use super::{Ptr, PtrMut, Usize};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "host")]
extern "C" {
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
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn physics_cast_ray(
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    max_toi: f32,
    filter_ptr: Ptr<QueryFilter>,
    out: PtrMut<CastRayResult>,
) -> u32 {
    let _ = (
        origin_x,
        origin_y,
        origin_z,
        direction_x,
        direction_y,
        direction_z,
        max_toi,
        filter_ptr,
        out,
    );
    panic!("`physics_cast_ray` is not implemented on this target");
}

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
