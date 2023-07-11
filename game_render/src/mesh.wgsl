struct CameraProjection {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct MeshMatrix {
    //mat: mat4x4<f32>,
    transform_0: vec4<f32>,
    transform_1: vec4<f32>,
    transform_2: vec4<f32>,
    transform_3: vec4<f32>,
    // This is not a mat3x3 so that alignment is not fucked.
    normal_0: vec4<f32>,
    normal_1: vec4<f32>,
    normal_2: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraProjection;
@group(1) @binding(0)
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
    @location(6) tbn_0: vec3<f32>,
    @location(7) tbn_1: vec3<f32>,
    @location(8) tbn_2: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    let transform = mat4x4(mesh.transform_0, mesh.transform_1, mesh.transform_2, mesh.transform_3);
    let normal_matrix = mat3x3(mesh.normal_0.xyz, mesh.normal_1.xyz, mesh.normal_2.xyz);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * transform * vec4<f32>(model.position, 1.0);

    let world_position = transform * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    
    // Normal
    let normal = normalize(normal_matrix * model.normal);
    let tangent = normalize(normal_matrix * model.tangent);
    let bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = mat3x3(
        tangent,
        bitangent,
        normal,
    );

    out.tbn_0 = tangent_matrix.x;
    out.tbn_1 = tangent_matrix.y;
    out.tbn_2 = tangent_matrix.z;

    let light_pos = vec3(-1.0, 0.0, 0.0);
    out.tangent_light_pos = tangent_matrix * light_pos;
    out.tangent_view_pos = tangent_matrix * camera.position.xyz;
    out.tangent_pos = tangent_matrix * world_position.xyz;

    out.world_normal = normalize(normal_matrix * model.normal);

    out.uv = model.uv;

    return out;
}
