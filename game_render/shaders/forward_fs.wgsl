struct MaterialConstants {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Camera {
    position: vec3<f32>,
    view_proj: mat4x4<f32>,
}

@group(2) @binding(0)
var<uniform> constants: MaterialConstants;
@group(2) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(2) @binding(2)
var normal_texture: texture_2d<f32>;
@group(2) @binding(3)
var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(4)
var linear_sampler: sampler;

@group(3) @binding(0)
var<storage> directional_lights: DirectionalLights;
@group(3) @binding(1)
var<storage> point_lights: PointLights;
@group(3) @binding(2)
var<storage> spot_lights: SpotLights;

struct FragInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec4<f32>,
}

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    var color = constants.base_color * textureSample(base_color_texture, linear_sampler, in.uv);

    var light_strength: vec3<f32> = vec3(0.0);
    for (var i: u32 = 0u; i < directional_lights.count; i++) {
        light_strength += compute_directional_light(in, directional_lights.lights[i]);
    }

    for (var i: u32 = 0u; i < point_lights.count; i++) {
        light_strength += compute_point_light(in, point_lights.lights[i]);
    }

    for (var i: u32 = 0u; i < spot_lights.count; i++) {
        light_strength += compute_spot_light(in, spot_lights.lights[i]);
    }

    color.r *= light_strength.r;
    color.g *= light_strength.g;
    color.b *= light_strength.b;
    return color;
}

fn compute_directional_light(in: FragInput, light: DirectionalLight) -> vec3<f32> {
    let normal = get_normal(in);

    let light_dir = normalize(-light.direction);

    let ambient = 0.05;

    let diffuse = max(dot(normal, light_dir), 0.0);

    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);

    return (ambient + diffuse + specular) * light.color;
}

fn compute_point_light(in: FragInput, light: PointLight) -> vec3<f32> {
    let normal = get_normal(in);

    let distance = length(light.position - in.world_position);
    let attenuation = 1.0 / (0.1 * distance);

    let light_dir = normalize(light.position - in.world_position);

    let ambient = 0.05;

    let diffuse = max(dot(normal, light_dir), 0.0);

    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);

    return ((ambient + diffuse + specular) * attenuation) * light.color;
}

fn compute_spot_light(in: FragInput, light: SpotLight) -> vec3<f32> {
    let normal = get_normal(in);

    let distance = length(light.position - in.world_position);
    let attenuation = 1.0 / (0.1 * distance);

    let light_dir = normalize(light.position - in.world_position);

    let ambient = 0.05;

    var diffuse = max(dot(normal, light_dir), 0.0);

    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    var specular = pow(max(dot(normal, half_dir), 0.0), 32.0);

    // Falloff
    // TODO: cosine can be precomputed on CPU side.
    let cos_outer = cos(light.outer_cutoff);
    let cos_inner = cos(light.inner_cutoff);

    let theta = dot(light_dir, -light.direction);

    let epsilon = cos_inner - cos_outer;
    let intensity = clamp((theta - cos_outer) / epsilon, 0.0, 1.0);
    diffuse *= intensity;
    specular *= intensity;

    return ((ambient + diffuse + specular) * attenuation) * light.color;
}

struct DirectionalLights {
    count: u32,
    lights: array<DirectionalLight>,
}

struct DirectionalLight {
    direction: vec3<f32>,
    color: vec3<f32>,
}

struct PointLights {
    count: u32,
    lights: array<PointLight>,
}

struct PointLight {
    position: vec3<f32>,
    color: vec3<f32>,
}

struct SpotLights {
    count: u32,
    lights: array<SpotLight>,
}

struct SpotLight {
    position: vec3<f32>,
    direction: vec3<f32>,
    color: vec3<f32>,
    inner_cutoff: f32,
    outer_cutoff: f32,
}

fn get_normal(in: FragInput) -> vec3<f32> {
    let normal_norm = normalize(in.world_normal);
    let tangent_norm = normalize(in.world_tangent.xyz);
    let bitangent = cross(normal_norm, tangent_norm) * in.world_tangent.w;
    let tbn = mat3x3(tangent_norm, bitangent, normal_norm);

    var normal = textureSample(normal_texture, linear_sampler, in.uv).rgb;
    normal = normalize(normal * 2.0 - 1.0);
    normal = normalize(tbn * normal);

    return normal;
}
