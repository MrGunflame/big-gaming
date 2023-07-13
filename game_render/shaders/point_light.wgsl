@group(0) @binding(0)
var g_position: texture_2d<f32>;
@group(0) @binding(1)
var g_albedo: texture_2d<f32>;
@group(0) @binding(2)
var g_normal: texture_2d<f32>;
@group(0) @binding(3)
var g_metallic_roughness: texture_2d<f32>;
@group(0) @binding(4)
var g_sampler: sampler;

@group(1) @binding(0)
var<uniform> light: PointLight;

@group(2) @binding(0)
var<uniform> camera: Camera;

struct PointLight {
    position: vec3<f32>,
    color: vec3<f32>,
}

struct Camera {
    position: vec3<f32>,
    view_proj: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = vec4(in.position.x, in.position.y, 0.0, 1.0);
    out.uv = in.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let position = textureSample(g_position, g_sampler, in.uv).xyz;
    let albedo = textureSample(g_albedo, g_sampler, in.uv).xyz;
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;
    let metallic = textureSample(g_metallic_roughness, g_sampler, in.uv).b;
    let roughness = textureSample(g_metallic_roughness, g_sampler, in.uv).g;

    let distance = length(light.position - position);
    let linear = 0.1;
    let attenuation = 1.0 / (linear * distance);

    let ambient = 0.0;

    let light_dir = normalize(light.position - position);

    let view_dir = normalize(camera.position - position);
    let half_dir = normalize(view_dir + light_dir);

    let reflect_dir = reflect(-light_dir, normal);

    let diffuse = max(dot(normal, light_dir), 0.0);
    let specular = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);

    let strength = light.color * (ambient + diffuse + specular) * attenuation;
    return vec4(strength * albedo, 1.0);

    //return vec4(normal, 1.0);
}
