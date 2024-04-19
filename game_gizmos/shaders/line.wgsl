@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<storage> lines: array<Vertex>;

struct Vertex {
    position: vec3<f32>,
    color: vec4<f32>,
}

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct Camera {
    position: vec3<f32>,
    view_proj: mat4x4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let vertex = lines[in.instance_index * 2u + in.vertex_index];

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    out.color = vertex.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
