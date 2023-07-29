struct Camera {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct Model {
    transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var<uniform> model: Model;

// Note that the storage buffers are dense, hence they are
// array<T, N>, not vecN<T>, so they are aligned to T.
@group(1) @binding(0)
var<storage> positions: array<array<f32, 3>>;
@group(1) @binding(1)
var<storage> normals: array<array<f32, 3>>;
@group(1) @binding(2)
var<storage> tangents: array<array<f32, 4>>;
@group(1) @binding(3)
var<storage> uvs: array<array<f32, 2>>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let position = fetch_position(in.vertex_index);
    let uv = fetch_uv(in.vertex_index);

    out.clip_position = camera.view_proj * model.transform * vec4<f32>(position, 1.0);
    out.uv = uv;

    return out;
}

fn fetch_position(vertex_index: u32) -> vec3<f32> {
   let x = positions[vertex_index][0];
   let y = positions[vertex_index][1];
   let z = positions[vertex_index][2];

   return vec3<f32>(x, y, z);
}

fn fetch_normal(vertex_index: u32) -> vec3<f32> {
    let x = normals[vertex_index][0];
    let y = normals[vertex_index][1];
    let z = normals[vertex_index][2];

    return vec3<f32>(x, y, z);
}

fn fetch_tangent(vertex_index: u32) -> vec4<f32> {
    let x = tangents[vertex_index][0];
    let y = tangents[vertex_index][1];
    let z = tangents[vertex_index][2];
    let w = tangents[vertex_index][3];

    return vec4<f32>(x, y, z, w);
}

fn fetch_uv(vertex_index: u32) -> vec2<f32> {
    let x = uvs[vertex_index][0];
    let y = uvs[vertex_index][1];

    return vec2<f32>(x, y);
}
