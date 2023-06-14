struct CameraProjection {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct MeshMatrix {
    mat: mat4x4<f32>,
    normal: mat3x3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraProjection;
@group(0) @binding(1)
var<uniform> mesh: MeshMatrix;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent_light_pos: vec3<f32>,
    @location(4) tangent_view_pos: vec3<f32>,
    @location(5) tangent_pos: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * mesh.mat * vec4<f32>(model.position, 1.0);

    let world_position = mesh.mat * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    
    // Normal
    let normal = normalize(mesh.normal * model.normal);
    let tangent = normalize(mesh.normal * model.tangent);
    let bitangent = normalize(mesh.normal * model.bitangent);
    let tangent_matrix = transpose(mat3x3(
        tangent,
        bitangent,
        normal,
    ));

    let light_pos = vec3(-1.0, 0.0, 0.0);
    out.tangent_light_pos = tangent_matrix * light_pos;
    out.tangent_view_pos = tangent_matrix * camera.position.xyz;
    out.tangent_pos = tangent_matrix * world_position.xyz;

    // out.world_normal = normalize(world_normal.xyz);
    
    out.uv = model.uv;

    return out;
}
