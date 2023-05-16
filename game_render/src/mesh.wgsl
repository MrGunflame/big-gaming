struct CameraProjection {
    view_proj: mat4x4<f32>,
};

struct MeshMatrix {
    mat: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraProjection;
@group(0) @binding(1)
var<uniform> mesh: MeshMatrix;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * mesh.mat * vec4<f32>(model.position, 1.0);

    let world_position = mesh.mat * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    
    // Normal
    let world_normal = mesh.mat * vec4<f32>(model.normal, 0.0);
    out.world_normal = normalize(world_normal.xyz);
    
    out.uv = model.uv;
    return out;
}
