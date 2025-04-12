#include pbr_params.wgsl

// PBR implementation based mostly on
// https://google.github.io/filament/Filament.html
//
// The Filament docs use some f16 optimizations sometimes
// that are not used here since they are mostly not vectorized
// and should not have much effect on desktop.
// Also we avoid mediump.

const PI: f32 = 3.14159265358979323846264338327950288;

/// Input material
struct Material {
    /// Base color in linear RGB
    base_color: vec4<f32>,
    /// Normal vector
    normal: vec3<f32>,
    /// Metallic factor
    metallic: f32,
    /// Perceptual roughness
    roughness: f32,
    /// Default 0.5
    reflectance: f32,
    /// Color factor of specular reflections.
    /// 
    /// Defaults to [1.0, 1.0, 1.0].
    specular_color: vec3<f32>,
    /// Factor of the specular reflection strength.
    ///
    /// Defaults to 1.0.
    specular_strength: f32,
}

struct Light {
    color: vec3<f32>,
    attenuation: f32,
    direction: vec3<f32>,
}

struct FragParams {
    diffuse_color: vec3<f32>,
    normal: vec3<f32>,
    /// Linear roughness (not perceptual)
    roughness: f32,
    metallic: f32,
    f0: vec3<f32>,
    f90: f32,
}

fn surface_shading(material: Material, view_dir: vec3<f32>, light: Light) -> vec3<f32> {
    let half_dir = normalize(view_dir + light.direction);
    
    let NoV = abs(dot(material.normal, view_dir)) + 1e-5;
    let NoL = clamp(dot(material.normal, light.direction), 0.0, 1.0);
    let NoH = clamp(dot(material.normal, half_dir), 0.0, 1.0);
    let LoH = clamp(dot(light.direction, half_dir), 0.0, 1.0);

    let params = compute_params(material);

    let Fr = specular_lobe(params, NoH, NoV, NoL, LoH);
    let Fd = diffuse_lobe(params,NoV, NoL, LoH);

    // TODO: Pixel energy compensation.
    // https://google.github.io/filament/Filament.html#materialsystem/improvingthebrdfs
    let color = Fd + Fr;

    return (color * light.color) * (light.attenuation * NoL);
}

fn compute_params(material: Material) -> FragParams {
    var frag: FragParams;
    frag.diffuse_color = compute_diffuse_color(material.base_color, material.metallic);
    frag.normal = material.normal;
    frag.roughness = perceptual_roughness_to_roughness(material.roughness);
    frag.metallic = material.metallic;

    let reflectance = compute_dielectic_f0(material.reflectance);

    let dielectric_specular_f0 = min(reflectance * material.specular_color, vec3(1.0)) * material.specular_strength;
    let dielectric_specular_f90 = material.specular_strength;

    // If specular_factor and specular_color_factor is at default 1.0, then
    // dielectric_specular_f0 = reflectance and this term becomes the standard
    // f0 term described here:
    // https://google.github.io/filament/Filament.html#materialsystem/specularbrdf/fresnel(specularf)
    frag.f0 = material.base_color.rgb * material.metallic + dielectric_specular_f0 * (1.0 - material.metallic);

    // FIXME: How is this derived?
    // For a material without specular filament uses a value of
    // f90 = saturate(f0, vec3(50 * 0.33)), but not if specular is enabled.
    // Instead this value will use f90 = 1.0 for the standard specular factor of 1.0,
    // which is what filament uses for low quality:
    // https://github.com/google/filament/blob/main/shaders/src/surface_brdf.fs#L173
    // https://github.com/google/filament/blob/main/shaders/src/surface_shading_model_standard.fs#L66
    frag.f90 = material.metallic + dielectric_specular_f90 * (1.0 - material.metallic);

    return frag;
}

fn D_GGX(NoH: f32, roughness: f32) -> f32 {
    let a = NoH * roughness;
    let k = roughness / (1.0 - NoH * NoH + a * a);
    return k * k * (1.0 / PI);
}

fn V_SmithGGXCorrelated(NoV: f32, NoL: f32, roughness: f32) -> f32 {
    let a2 = roughness * roughness;
    let ggx_v = NoL * sqrt(NoV * NoV * (1.0 - a2) + a2);
    let ggx_l = NoV * sqrt(NoL * NoL * (1.0 - a2) + a2);
    return 0.5 / (ggx_v + ggx_l);
}

fn F_Schlick(u: f32, f0: vec3<f32>, f90: f32) -> vec3<f32> {
    return f0 + (vec3<f32>(f90) - f0) * pow(1.0 - u, 5.0);
}

fn fresnel(LoH: f32, f0: vec3<f32>) -> vec3<f32> {
    let f90 = saturate(dot(f0, vec3(50.0 * 0.33)));
    return F_Schlick(LoH, f0, f90);
}

fn fresnel_f90(LoH: f32, f0: vec3<f32>, f90: f32) -> vec3<f32> {
    return F_Schlick(LoH, f0, f90);
}

fn Fd_Burley(NoV: f32, NoL: f32, LoH: f32, roughness: f32) -> vec3<f32> {
    let f90 = 0.5 + 2.0 * roughness * LoH * LoH;
    let light_scatter = F_Schlick(NoL, vec3(1.0), f90);
    let view_scatter = F_Schlick(NoV, vec3(1.0), f90);
    return light_scatter * view_scatter * (1.0 / PI);
}

fn diffuse_lobe(params: FragParams, NoV: f32, NoL: f32, LoH: f32) -> vec3<f32> {
    return params.diffuse_color * Fd_Burley(NoV, NoL, LoH, params.roughness);
}

fn specular_lobe(params: FragParams, NoH: f32, NoV: f32, NoL: f32, LoH: f32) -> vec3<f32> {
    return isotropic_lobe(params, NoH, NoV, NoL, LoH);
}

fn isotropic_lobe(params: FragParams, NoH: f32, NoV: f32, NoL: f32, LoH: f32) -> vec3<f32> {
    let D = D_GGX(NoH, params.roughness);
    let V = V_SmithGGXCorrelated(NoV, NoL, params.roughness);
    let F = fresnel_f90(LoH, params.f0, params.f90);
    // F = fresnel(f0, LoH);

    return (D * V) * F;
}
