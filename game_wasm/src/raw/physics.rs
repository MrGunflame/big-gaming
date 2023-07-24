use bytemuck::{Pod, Zeroable};

use super::PtrMut;

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
        out: PtrMut<CastRayResult>,
    ) -> u32;
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct CastRayResult {
    pub entity_id: u64,
    pub toi: f32,
    pub _pad0: u32,
}
