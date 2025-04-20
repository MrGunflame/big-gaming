#include pbr.wgsl

struct MaterialConstants {
    flags: u32,
    // pad0: [u32; 3]
    base_color: vec4<f32>,
    roughness: f32,
    metallic: f32,
    reflectance: f32,
    specular_strength: f32,
    specular_color: vec4<f32>,
}

const MATERIAL_FLAGS_UNLIT: u32 = 1u << 0;
const MATERIAL_FLAGS_FLIP_NORMAL_Y: u32 = 1u << 1;
const MATERIAL_FLAGS_METALLIC_FROM_SPECULAR: u32 = 1u << 2;
const MATERIAL_FLAGS_ROUGHNESS_FROM_GLOSSINESS: u32 = 1u << 3;
const MATERIAL_FLAGS_NORMAL_ENCODING_TWO_COMPONENT: u32 = 1u << 4;

var<push_constant> push_constants: PushConstants;

struct PushConstants {
    camera: Camera,
    // FIXME: Options almost never change. Instead of doing dynamic matching
    // in the shader we should just recompile the shader with only the paths
    // defined in the options.
    // This requires a shader pre-processor which we currently do not have.
    // options: Options,
}

struct Options {
    shading_mode: u32,
}

const SHADING_MODE_FULL: u32 = 0u;
const SHADING_MODE_ALBEDO: u32 = 1u;
const SHADING_MODE_NORMAL: u32 = 2u;
const SHADING_MODE_TANGENT: u32 = 3u;

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
var specular_glossiness_texture: texture_2d<f32>;

@group(3) @binding(0)
var<storage> directional_lights: DirectionalLights;
@group(3) @binding(1)
var<storage> point_lights: PointLights;
@group(3) @binding(2)
var<storage> spot_lights: SpotLights;
@group(3) @binding(3)
var linear_sampler: sampler;

struct FragInput {
    @builtin(position) clip_position: vec4<f32>,
    @builtin(front_facing) front_facing: bool,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec4<f32>,
}

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    var color = constants.base_color * textureSample(base_color_texture, linear_sampler, in.uv);
    color.a = 1.0;

    if (constants.flags & MATERIAL_FLAGS_UNLIT) != 0 {
        return color;
    }

    let specular_strength = constants.specular_strength * get_specular_strength(in);
    let specular_color = constants.specular_color.rgb;

    var roughness = get_roughness(in);
    if (constants.flags & MATERIAL_FLAGS_ROUGHNESS_FROM_GLOSSINESS) != 0 {
        roughness = get_roughness_from_glossiness(in);
    }

    var metallic = get_metallic(in);
    if (constants.flags & MATERIAL_FLAGS_METALLIC_FROM_SPECULAR) != 0 {
        metallic = compute_metallic_from_specular_color(specular_color * specular_strength);
    }

    // BRDF parameters
    let view_dir = normalize(push_constants.camera.position - in.world_position);
    var material: Material;
    material.base_color = get_base_color(in);
    material.normal = get_normal(in);
    material.metallic = metallic;
    material.roughness = roughness;
    material.reflectance = constants.reflectance;
    material.specular_color = specular_color;
    material.specular_strength = specular_strength;

    var luminance: vec3<f32> = vec3(0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < directional_lights.count; i++) {
        let light = compute_directional_light(in, directional_lights.lights[i]);
        luminance += surface_shading(material, view_dir, light);
    }

    for (var i: u32 = 0u; i < point_lights.count; i++) {
        let light = compute_point_light(in, point_lights.lights[i]);
        luminance += surface_shading(material, view_dir, light);
    }

    for (var i: u32 = 0u; i < spot_lights.count; i++) {
        let light = compute_spot_light(in, spot_lights.lights[i]);
        luminance += surface_shading(material, view_dir, light);
    }

    return vec4<f32>(luminance.r, luminance.g, luminance.b, color.a);
}

fn compute_directional_light(in: FragInput, light: DirectionalLight) -> Light {
    let light_dir = normalize(-light.direction);

    var l: Light;
    l.color = light.color * light.intensity;
    l.attenuation = 1.0;
    l.direction = light_dir;
    return l;
}

fn compute_point_light(in: FragInput, light: PointLight) -> Light {
    let distance = length(light.position - in.world_position);
    let pos_to_light = light.position - in.world_position;
    let attenuation = get_distance_attenuation(dot(pos_to_light, pos_to_light), (1.0 / light.radius) * (1.0 / light.radius));

    let light_dir = normalize(light.position - in.world_position);

    var l: Light;
    l.color = light.color * light.intensity;
    l.attenuation = attenuation;
    l.direction = light_dir;
    return l;
}

fn compute_spot_light(in: FragInput, light: SpotLight) -> Light {
    let pos_to_light = light.position - in.world_position;
    let attenuation = get_distance_attenuation(dot(pos_to_light, pos_to_light), (1.0 / light.radius) * (1.0 / light.radius));

    let light_dir = normalize(light.position - in.world_position);

    // Falloff
    // TODO: cosine can be precomputed on CPU side.
    let cos_outer = cos(light.outer_cutoff);
    let cos_inner = cos(light.inner_cutoff);

    let theta = dot(light_dir, -light.direction);

    let epsilon = cos_inner - cos_outer;
    let intensity = clamp((theta - cos_outer) / epsilon, 0.0, 1.0);

    var l: Light;
    l.color = light.color * light.intensity;
    l.attenuation = attenuation;
    l.direction = light_dir;
    return l;
}

struct DirectionalLights {
    count: u32,
    lights: array<DirectionalLight>,
}

struct DirectionalLight {
    direction: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

struct PointLights {
    count: u32,
    lights: array<PointLight>,
}

struct PointLight {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
}

struct SpotLights {
    count: u32,
    lights: array<SpotLight>,
}

struct SpotLight {
    position: vec3<f32>,
    direction: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
    inner_cutoff: f32,
    outer_cutoff: f32,
}

fn get_specular_strength(in: FragInput) -> f32 {
    return constants.specular_strength * textureSample(specular_glossiness_texture, linear_sampler, in.uv).r;
}

fn get_roughness_from_glossiness(in: FragInput) -> f32 {
    return constants.roughness * (1.0 - textureSample(specular_glossiness_texture, linear_sampler, in.uv).g);
}

fn get_base_color(in: FragInput) -> vec4<f32> {
    return constants.base_color * textureSample(base_color_texture, linear_sampler, in.uv);
}

fn get_albedo(in: FragInput) -> vec3<f32> {
    return constants.base_color.rgb * textureSample(base_color_texture, linear_sampler, in.uv).rgb;
}

fn get_normal(in: FragInput) -> vec3<f32> {
    let normal_norm = normalize(in.world_normal);
    let tangent_norm = normalize(in.world_tangent.xyz);
    let bitangent = cross(normal_norm, tangent_norm) * in.world_tangent.w;
    let tbn = mat3x3(tangent_norm, bitangent, normal_norm);

    var normal = textureSample(normal_texture, linear_sampler, in.uv).rgb;

    if (constants.flags & MATERIAL_FLAGS_FLIP_NORMAL_Y) != 0 {
        normal.g = 1.0 - normal.g;
    }

    normal = normal * 2.0 - 1.0;

    if (constants.flags & MATERIAL_FLAGS_NORMAL_ENCODING_TWO_COMPONENT) != 0 {
        normal.z = sqrt(1.0 - dot(normal.xy, normal.xy));
    }

    normal = normalize(normal);
    normal = normalize(tbn * normal);

    // Invert the normal if the triangle is facing backwards.
    // Without this step the lighting direction will be inversed
    // for back-facing triangles.
    if in.front_facing {
        return normal;
    } else {
        return -normal;
    }
}

fn get_roughness(in: FragInput) -> f32 {
    return constants.roughness * textureSample(metallic_roughness_texture, linear_sampler, in.uv).g;
}

fn get_metallic(in: FragInput) -> f32 {
    return constants.metallic * textureSample(metallic_roughness_texture, linear_sampler, in.uv).b;
}

fn get_distance_attenuation(distance_square: f32, inv_range_squared: f32) -> f32 {
    let factor = distance_square * inv_range_squared;
    let smooth_factor = saturate(1.0 - factor * factor);
    let attenuation = smooth_factor * smooth_factor;
    return attenuation / max(distance_square, 0.0001);
}
