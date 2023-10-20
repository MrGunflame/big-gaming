struct Light {
    projection: mat4x4<f32>,
}

struct Model {
    transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> light: Light;

@group(0) @binding(1)
var<uniform> model: Model;

@group(1) @binding(0)
var<storage> positions: array<array<f32, 3>>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let position = fetch_position(in.vertex_index);
    out.clip_position = light.projection * model.transform * vec4<f32>(position, 1.0);
    return out;
}


fn fetch_position(vertex_index: u32) -> vec3<f32> {
    let x = positions[vertex_index][0];
    let y = positions[vertex_index][1];
    let z = positions[vertex_index][2];
    return vec3<f32>(x, y, z);
}
