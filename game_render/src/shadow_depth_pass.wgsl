
@group(0) @binding(0)
var<uniform> light_space_matrix: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> model: mat4x4<f32>;


struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = light_space_matrix * model * vec4(in.position, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) {

}
