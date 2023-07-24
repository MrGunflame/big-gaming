use game_common::math::Ray;
use game_wasm::raw::physics::CastRayResult;
use glam::Vec3;
use wasmtime::Caller;

use crate::builtin::CallerExt;
use crate::instance::State;

pub fn physics_cast_ray(
    mut caller: Caller<'_, State<'_, '_>>,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    max_toi: f32,
    out: u32,
) -> wasmtime::Result<u32> {
    tracing::trace!("physics_cast_ray(origin_x = {}, origin_y = {}, origin_z = {}, direction_x = {}, direction_y = {}, direction_z = {}, max_toi = {})", origin_x, origin_y, origin_z, direction_x, direction_y, direction_z, max_toi);

    let ray = Ray {
        origin: Vec3::new(origin_x, origin_y, origin_z),
        direction: Vec3::new(direction_x, direction_y, direction_z),
    };

    let (entity_id, toi) = match caller.data().physics_pipeline.cast_ray(ray, max_toi) {
        Some((entity_id, toi)) => (entity_id, toi),
        None => return Ok(1),
    };

    caller.write(
        out,
        &CastRayResult {
            entity_id: entity_id.into_raw(),
            toi,
            _pad0: 0,
        },
    )?;

    Ok(0)
}
