struct MaterialConstants {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
}

@group(2) @binding(0)
var<uniform> constants: MaterialConstants;
@group(2) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(2) @binding(2)
var normal_texture: texture_2d<f32>;
@group(2) @binding(3)
var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(4)
var linear_sampler: sampler;

struct FragInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    let color = constants.base_color * textureSample(base_color_texture, linear_sampler, in.uv);

    return color;
}
