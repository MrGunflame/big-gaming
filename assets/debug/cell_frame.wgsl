@fragment
fn fragment(
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    if world_normal[1] == 0.0 {
        return vec4(0.0, 1.0, 0.0, 0.5);
    }

    if world_normal[0] == 0.0 {
        return vec4(1.0, 0.0, 0.0, 0.5);
    }

    return vec4(1.0, 0.0, 1.0, 0.0);
}
