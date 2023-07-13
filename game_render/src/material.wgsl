
@group(2) @binding(0)
var<uniform> base_color: vec4<f32>;
@group(2) @binding(1)
var color_texture: texture_2d<f32>;
@group(2) @ binding(2)
var color_texture_sampler: sampler;

@group(2) @binding(3)
var normal_texture: texture_2d<f32>;
@group(2) @binding(4)
var normal_sampler: sampler;
@group(2) @binding(5)
var<uniform> roughness: f32;
@group(2) @binding(6)
var<uniform> metallic: f32;
@group(2) @binding(7)
var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(8)
var metallic_roughness_sampler: sampler;

@group(0) @binding(0)
var<uniform> camera: CameraProjection;

struct CameraProjection {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tbn_0: vec3<f32>,
    @location(3) tbn_1: vec3<f32>,
    @location(4) tbn_2: vec3<f32>,
};

// GBuffer output
struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) albedo: vec4<f32>,
    @location(3) metallic_roughness: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> GBuffer {
    let tbn = mat3x3(in.tbn_0, in.tbn_1, in.tbn_2);

    var color: vec4<f32> = base_color * textureSample(color_texture, color_texture_sampler, in.uv);
    var normal = textureSample(normal_texture, normal_sampler, in.uv).xyz;
    let local_metallic = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv).b;
    let local_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv).g;

    //normal = normal * 2.0 - 1.0;
    normal = normalize(tbn * normal);

    var gbuffer: GBuffer;
    gbuffer.position = vec4(in.world_position, 1.0);
    gbuffer.normal = vec4(normal, 1.0);
    gbuffer.albedo = color;
    gbuffer.metallic_roughness.b = local_metallic * metallic;
    gbuffer.metallic_roughness.g = local_roughness * roughness;
    return gbuffer;
}
