
struct Material {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: Material;

@fragment
fn fragment(
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    let x = world_normal[0] * 1.0;
    let y = world_normal[1] * 0.0;
    let z = world_normal[2] * 1.0;

    //return vec4(x, y, z, 0.0);
    return vec4(0.0, 0.0, 0.0, 0.0);
}
