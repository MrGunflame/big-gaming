
@group(1) @binding(0)
var color_texture: texture_2d<f32>;
@group(1) @ binding(1)
var color_texture_sampler: sampler;


struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(color_texture, color_texture_sampler, in.uv);
}
