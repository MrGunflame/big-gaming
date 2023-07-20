use wasmtime::Caller;

use crate::instance::State;

pub fn physics_cast_ray(
    mut caller: Caller<'_, State<'_>>,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    max_toi: f32,
) {
    tracing::trace!("physics_cast_ray(origin_x = {}, origin_y = {}, origin_z = {}, direction_x = {}, direction_y = {}, direction_z = {}, max_toi = {})", origin_x, origin_y, origin_z, direction_x, direction_y, direction_z, max_toi);

    todo!()
}
