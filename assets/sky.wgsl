
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
    let y = world_normal[1] * 1.0;
    let z = world_normal[2] * 1.0;

    let origin = vec2(0.0, 0.0);

    let origin = vec3(0.0, 0.0, 0.0);

    return material.color;
    //return vec4(x, y, z, 0.5);
    // return vec4(0.0, 0.0, 0.0, 0.0);
    //return vec4(color[0], color[1], color[2], 0.0);
    // return vec4(world_position[0], world_position[1], world_position[2], 0.0);
}
