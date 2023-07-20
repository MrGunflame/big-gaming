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
    );
}
