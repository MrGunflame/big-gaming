@group(0) @binding(0)
var g_position: texture_2d<f32>;
@group(0) @binding(1)
var g_sampler: sampler;
@group(0) @binding(2)
var g_normal: texture_2d<f32>;
@group(0) @binding(3)
var g_albedo: texture_2d<f32>;
@group(0) @binding(4)
var<uniform> camera: CameraProjection;

struct CameraProjection {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

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
    let pos = textureSample(g_position, g_sampler, in.uv);
    let normal = textureSample(g_normal, g_sampler, in.uv);
    let albedo = textureSample(g_albedo, g_sampler, in.uv);

    var l: DirectionalLight;
    l.direction = vec3(1.0, 0.2, 0.4);
    l.color = vec3(1.0, 1.0, 1.0);
    l.ambient = 0.01;
    l.diffuse = 1.0;
    l.specular = 1.0;
    let light = directional_light(in, l);

    let color = vec4(albedo.xyz, 1.0) * light;

    return color;
}

struct DirectionalLight {
    direction: vec3<f32>,
    color: vec3<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
}

fn directional_light(in: VertexOutput, light: DirectionalLight) -> vec4<f32> {
    let pos = textureSample(g_position, g_sampler, in.uv).xyz;
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;
    
    //let light_pos = normalize(-light.direction);

    //let light_dir = normalize(in.tangent_light_pos - in.tangent_pos);
    let light_dir = normalize(light.direction);

    // Ambient
    let ambient = light.color * light.ambient;

    // Diffuse
    let diffuse_strength = max(dot(normal, light_dir), 0.0);
    let diffuse = light.color * diffuse_strength;

    // Specular
    let view_dir = normalize(camera.position.xyz - pos);
    let half_dir = normalize(view_dir + light_dir);

    let specular_strength = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let specular = light.color * specular_strength;

    return vec4(ambient + diffuse + specular, 1.0);
}