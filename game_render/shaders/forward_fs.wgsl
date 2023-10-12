const PI: f32 = 3.14159265358979323846264338327950288;

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

    var luminance: vec3<f32> = vec3(0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < directional_lights.count; i++) {
        luminance += compute_directional_light(in, directional_lights.lights[i]);
    }

    for (var i: u32 = 0u; i < point_lights.count; i++) {
        luminance += compute_point_light(in, point_lights.lights[i]);
    }

    for (var i: u32 = 0u; i < spot_lights.count; i++) {
        luminance += compute_spot_light(in, spot_lights.lights[i]);
    }

    return vec4<f32>(luminance.r, luminance.g, luminance.b, color.a);
}

fn compute_directional_light(in: FragInput, light: DirectionalLight) -> vec3<f32> {
    let normal = get_normal(in);

    let light_dir = normalize(-light.direction);

    let ambient = 0.05;

    let diffuse = max(dot(normal, light_dir), 0.0);

    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);

    //return (ambient + diffuse + specular) * light.intensity * light.color;
    //let NoL = clamp(dot(normal, light_dir), 0.0, 1.0);
    //let illuminance = light.intensity * NoL;

    //return brdf(in, light_dir) * light.color * illuminance;

    var l: Light;
    l.color = light.color;
    l.color.r *= light.intensity;
    l.color.g *= light.intensity;
    l.color.b *= light.intensity;
    l.attenuation = 1.0;
    l.direction = light_dir;
    return surface_shading(in, l);
}

fn compute_point_light(in: FragInput, light: PointLight) -> vec3<f32> {
    let normal = get_normal(in);

    let distance = length(light.position - in.world_position);
    let pos_to_light = light.position - in.world_position;

    let attenuation = get_distance_attenuation(dot(pos_to_light, pos_to_light), (1.0 / light.radius) * (1.0 / light.radius));

    let light_dir = normalize(light.position - in.world_position);

    let ambient = 0.00;

    let diffuse = max(dot(normal, light_dir), 0.0);

    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);

    //let NoL = clamp(dot(normal, light_dir), 0.0, 1.0);
    //return (brdf(in, light_dir) * light.intensity * attenuation * NoL) * light.color;

    var l: Light;
    l.color = light.color;
    l.color.r *= light.intensity;
    l.color.g *= light.intensity;
    l.color.b *= light.intensity;
    l.attenuation = attenuation;
    l.direction = light_dir;

    //return ((ambient + diffuse + specular) * light.intensity * attenuation) * light.color;
    return surface_shading(in, l);
}

fn compute_spot_light(in: FragInput, light: SpotLight) -> vec3<f32> {
    let normal = get_normal(in);

    let pos_to_light = light.position - in.world_position;
    let attenuation = get_distance_attenuation(dot(pos_to_light, pos_to_light), (1.0 / light.radius) * (1.0 / light.radius));

    let light_dir = normalize(light.position - in.world_position);

    let ambient = 0.00;

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

    var l: Light;
    l.color = light.color;
    l.color.r *= light.intensity;
    l.color.g *= light.intensity;
    l.color.b *= light.intensity;
    l.attenuation = attenuation;
    l.direction = light_dir;

    return surface_shading(in, l);
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

fn get_albedo(in: FragInput) -> vec3<f32> {
    return constants.base_color.rgb * textureSample(base_color_texture, linear_sampler, in.uv).rgb;
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

// PBR implementation based on
// https://google.github.io/filament/Filament.html

// fn D_GGX(NoH: f32, roughness: f32) -> f32 {
//     let a = NoH * roughness;
//     let k = roughness / (1.0 - NoH * NoH + a * a);
//     return k * k * (1.0 / PI);
// }

// fn V_SmithGGXCorrelated(NoV: f32, NoL: f32, roughness: f32) -> f32 {
//     let a2 = roughness * roughness;
//     let GGXV = NoL * sqrt(NoV * NoV * (1.0 - a2) + a2);
//     let GGXL = NoV * sqrt(NoL * NoL * (1.0 - a2) + a2);
//     return 0.5 / (GGXV + GGXL);
// }

// fn Fd_Lambert() -> f32 {
//     return 1.0 / PI;
// }

// fn brdf(in: FragInput, light_dir: vec3<f32>, ) -> vec3<f32> {
//     let albedo = get_albedo(in);
//     let normal = get_normal(in);
//     let roughness = get_roughness(in);
//     let metallic = get_metallic(in);

//     let view_dir = normalize(camera.position - in.world_position);
//     let half_dir = normalize(view_dir + light_dir);

//     let NoV = abs(dot(normal, view_dir)) + 1e-5;
//     let NoL = clamp(dot(normal, light_dir), 0.0, 1.0);
//     let NoH = clamp(dot(normal, half_dir), 0.0, 1.0);
//     let LoH = clamp(dot(light_dir, half_dir), 0.0, 1.0);

//     let a = roughness * roughness;
//     var f0 = vec3(0.04);
//     f0 = mix(f0, albedo, metallic);

//     // let D = D_GGX(NoH, a);
//     // let F = F_Schlick(LoH, f0);
//     // let V = V_SmithGGXCorrelated(NoV, NoL, roughness);

//     // Specular BRDF
//     //let Fr = (D * V) * F;
//     // Diffuse BRDF
//     //let Fd = albedo * Fd_Lambert();

//     let spec = specular(f0, roughness, half_dir, NoV, NoL, NoH, LoH);
//     //let diffuse = Fd_Burley(roughness, NoV, NoL, LoH) * albedo;
//     return spec;

//     //return Fd + Fr;
// }

// fn specular(f0: vec3<f32>, roughness: f32, half_dir: vec3<f32>, NoV: f32, NoL: f32, NoH: f32, LoH: f32) -> vec3<f32> {
//     let D = D_GGX(NoH, roughness);
//     let V = V_SmithGGXCorrelated(NoV, NoL, roughness);
//     let F = fresnel(f0, LoH);

//     let Fr = D * V * F;
//     return Fr;
// }

fn surface_shading(in: FragInput, light: Light) -> vec3<f32> {
    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light.direction);

    let normal = get_normal(in);

    let NoV = abs(dot(normal, view_dir));
    let NoL = clamp(dot(normal, light.direction), 0.0, 1.0);
    let NoH = clamp(dot(normal, half_dir), 0.0, 1.0);
    let LoH = clamp(dot(light.direction, half_dir), 0.0, 1.0);

    let Fr = specular_color(in, half_dir, NoV, NoL, NoH, LoH);
    let Fd = diffuse_color(in, NoV, NoL, LoH);

    let color = Fd + Fr;

    return (color * light.color) * (light.attenuation * NoL);
}

// ---
// --- Specular impl
// ---

const MEDIUM_FLT_MAX: f32 = 65504.0;
fn saturate_medium_p(x: f32) -> f32 {
    return min(x, MEDIUM_FLT_MAX);
}

fn D_GGX(roughness: f32, NoH: f32, h: vec3<f32>) -> f32 {
    let one_minus_noh_squared = 1.0 - NoH * NoH;

    let a = NoH * roughness;
    let k = roughness / (one_minus_noh_squared + a * a);
    let d = k * k * (1.0 / PI);
    return saturate_medium_p(d);
}

fn V_SmithGGXCorrelated(roughness: f32, NoV: f32, NoL: f32) -> f32 {
    let a2 = roughness * roughness;
    let lambda_v = NoV * sqrt((NoV - a2 * NoV) * NoV + a2);
    let lambda_l = NoL * sqrt((NoL - a2 * NoL) * NoL + a2);
    let v = 0.5 / (lambda_v + lambda_l);
    return saturate_medium_p(v);
}

fn distribution(roughness: f32, NoH: f32, h: vec3<f32>) -> f32 {
    return D_GGX(roughness, NoH, h);
}

fn visibility(roughness: f32, NoV: f32, NoL: f32) -> f32 {
    return V_SmithGGXCorrelated(roughness, NoV, NoL);
}

fn fresnel(f0: vec3<f32>, LoH: f32) -> vec3<f32> {
    let f90 = saturate(dot(f0, vec3<f32>(50.0 * 0.33)));
    return F_Schlick_vec3(f0, f90, LoH);
}

fn F_Schlick_vec3(f0: vec3<f32>, f90: f32, VoH: f32) -> vec3<f32> {
    return f0 + (f90 - f0) * pow(1.0 - VoH, 5.0);
}

fn F_Schlick(f0: f32, f90: f32, VoH: f32) -> f32 {
    return f0 + (f90 - f0) * pow(1.0 - VoH, 5.0);
}

fn specular_color(in: FragInput, h: vec3<f32>, NoV: f32, NoL: f32, NoH: f32, LoH: f32) -> vec3<f32> {
    return isotropic(in, h, NoV, NoL, NoH, LoH);
}

fn isotropic(in: FragInput, h: vec3<f32>, NoV: f32, NoL: f32, NoH: f32, LoH: f32) -> vec3<f32> {
    let albedo = get_albedo(in);
    let roughness = get_roughness(in);
    let metallic = get_metallic(in);

    var f0 = vec3(0.04);
    f0 = mix(f0, albedo, metallic);

    let d = distribution(roughness, NoH, h);
    let v = visibility(roughness, NoV, NoL);
    let f = fresnel(f0, LoH);

    return (d * v) * f;
}

// ----------------
// --- Diffuse impl
// ----------------

fn Fd_Burley(roughness: f32, NoV: f32, NoL: f32, LoH: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * LoH * LoH;
    let light_scatter = F_Schlick(1.0, f90, NoL);
    let view_scatter = F_Schlick(1.0, f90, NoV);
    return light_scatter * view_scatter * (1.0 / PI);
}

fn diffuse(roughness: f32, NoV: f32, NoL: f32, LoH: f32) -> f32 {
    return Fd_Burley(roughness, NoV, NoL, LoH);
}

fn diffuse_color(in: FragInput, NoV: f32, NoL: f32, LoH: f32) -> vec3<f32> {
    let color = get_albedo(in);
    let roughness = get_roughness(in);
    return color * diffuse(roughness, NoV, NoL, LoH);
}

struct Light {
    color: vec3<f32>,
    attenuation: f32,
    direction: vec3<f32>,
}
