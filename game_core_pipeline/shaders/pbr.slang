module pbr;

import types;

public namespace pbr {
    static const f32 PI = f32.getPi();

    /// Material properties used for PBR shading.
    public struct Material {
        /// Base color of the material in RGB.
        public Vec3 base_color;
        /// Normal vector
        public Vec3 normal;
        /// Metallic factor
        ///
        /// Defaults to `0.0`.
        public f32 metallic;
        /// Perceptual roughness
        ///
        /// Defaults to `0.5`.
        public f32 roughness;
        /// Defaults to `0.5`.
        public f32 reflectance;
        /// Color factor of specular reflections.
        ///
        /// Defaults to `[1.0, 1.0, 1.0]`.
        public Vec3 specular_color;
        /// Factor of the specular reflection strength.
        ///
        /// Defaults to `1.0`.
        public f32 specular_strength;

        public __init() {
            this.base_color = Vec3(0.0);
            this.normal = Vec3(0.0);
            this.metallic = 0.0;
            this.roughness = 0.5;
            this.reflectance = 0.5;
            this.specular_color = Vec3(1.0);
            this.specular_strength = 1.0;
        }
    }

    public struct Light {
        public Vec3 color;
        public f32 attenuation;
        public Vec3 direction;
    }

    public func surface_shading(material: Material, view_dir: Vec3, light: Light) -> Vec3 {
        let half_dir = normalize(view_dir + light.direction);
        
        let NoV = abs(dot(material.normal, view_dir)) + 1e-5;
        let NoL = clamp(dot(material.normal, light.direction), 0.0, 1.0);
        let NoH = clamp(dot(material.normal, half_dir), 0.0, 1.0);
        let LoH = clamp(dot(light.direction, half_dir), 0.0, 1.0);

        let params = FragParams(material);

        let Fr = specular_lobe(params, NoH, NoV, NoL, LoH);
        let Fd = diffuse_lobe(params,NoV, NoL, LoH);

        // TODO: Pixel energy compensation.
        // https://google.github.io/filament/Filament.html#materialsystem/improvingthebrdfs
        let color = Fd + Fr;

        return (color * light.color) * (light.attenuation * NoL);
    }

    struct FragParams {
        Vec3 diffuse_color;
        Vec3 normal;
        /// Linear roughness
        f32 roughness;
        f32 metallic;
        Vec3 f0;
        f32 f90;

        __init(Material material) {
            this.diffuse_color = compute_diffuse_color(material.base_color, material.metallic);
            this.normal = material.normal;
            this.roughness = perceptual_roughness_to_roughness(material.roughness);
            this.metallic = material.metallic;

            let reflectance = compute_dielectric_f0(material.reflectance);

            let dielectric_specular_f0 = min(reflectance * material.specular_color, Vec3(1.0)) * material.specular_strength;
            let dielectric_specular_f90 = material.specular_strength;

            // If specular_factor and specular_color_factor is at default 1.0, then
            // dielectric_specular_f0 = reflectance and this term becomes the standard
            // f0 term described here:
            // https://google.github.io/filament/Filament.html#materialsystem/specularbrdf/fresnel(specularf)
            this.f0 = material.base_color.rgb * material.metallic + dielectric_specular_f0 * (1.0 - material.metallic);

            // FIXME: How is this derived?
            // For a material without specular filament uses a value of
            // f90 = saturate(f0, vec3(50 * 0.33)), but not if specular is enabled.
            // Instead this value will use f90 = 1.0 for the standard specular factor of 1.0,
            // which is what filament uses for low quality:
            // https://github.com/google/filament/blob/main/shaders/src/surface_brdf.fs#L173
            // https://github.com/google/filament/blob/main/shaders/src/surface_shading_model_standard.fs#L66
            this.f90 = material.metallic + dielectric_specular_f90 * (1.0 - material.metallic);
        }
    }

    func compute_diffuse_color(base_color: Vec3, metallic: f32) -> Vec3 {
        return base_color * (1.0 - metallic);
    }


    func compute_f0(base_color: Vec3, metallic: f32, reflectance: f32) -> Vec3 {
        return base_color * metallic + (reflectance * (1.0 - metallic));
    }

    func compute_dielectric_f0(reflectance: f32) -> f32 {
        return 0.16 * reflectance * reflectance;
    }

    func perceptual_roughness_to_roughness(perceptual_roughness: f32) -> f32 {
        // We clamp the roughness to 0.089 to prevent precision problems.
        // We might be able to lower that value to 0.045 since we're using
        // 32-bit floats.
        // See https://google.github.io/filament/Filament.html#toc4.8.3.3
        let clamped_perceptual_roughness = clamp(perceptual_roughness, 0.089, 1.0);
        return clamped_perceptual_roughness * clamped_perceptual_roughness;
    }

    func D_GGX(NoH: f32, roughness: f32) -> f32 {
        let a = NoH * roughness;
        let k = roughness / (1.0 - NoH * NoH + a * a);
        return k * k * (1.0 / PI);
    }

    func V_SmithGGXCorrelated(NoV: f32, NoL: f32, roughness: f32) -> f32 {
        let a2 = roughness * roughness;
        let ggx_v = NoL * sqrt(NoV * NoV * (1.0 - a2) + a2);
        let ggx_l = NoV * sqrt(NoL * NoL * (1.0 - a2) + a2);
        return 0.5 / (ggx_v + ggx_l);
    }

    func F_Schlick(u: f32, f0: Vec3, f90: f32) -> Vec3 {
        return f0 + (Vec3(f90) - f0) * pow(1.0 - u, 5.0);
    }

    func fresnel(LoH: f32, f0: Vec3) -> Vec3 {
        let f90 =  saturate(dot(f0, Vec3(50.0 * 0.33)));
        return F_Schlick(LoH, f0, f90);
    }

    func fresnel_f90(LoH: f32, f0: Vec3, f90: f32) -> Vec3 {
        return F_Schlick(LoH, f0, f90);
    }

    func Fd_Burley(NoV: f32, NoL: f32, LoH: f32, roughness: f32) -> Vec3 {
        let f90 = 0.5 + 2.0 * roughness * LoH * LoH;
        let light_scatter = F_Schlick(NoL, Vec3(1.0), f90);
        let view_scatter = F_Schlick(NoV, Vec3(1.0), f90);
        return light_scatter * view_scatter * (1.0 / PI);
    }

    func diffuse_lobe(params: FragParams, NoV: f32, NoL: f32, LoH: f32) -> Vec3 {
        return params.diffuse_color * Fd_Burley(NoV, NoL, LoH, params.roughness);
    }

    func specular_lobe(params: FragParams, NoH: f32, NoV: f32, NoL: f32, LoH: f32) -> Vec3 {
        return isotropic_lobe(params, NoH, NoV, NoL, LoH);
    }

    func isotropic_lobe(params: FragParams, NoH: f32, NoV: f32, NoL: f32, LoH: f32) -> Vec3 {
        let D = D_GGX(NoH, params.roughness);
        let V = V_SmithGGXCorrelated(NoV, NoL, params.roughness);
        let F = fresnel_f90(LoH, params.f0, params.f90);
        // F = fresnel(f0, LoH);

        return (D * V) * F;
    }
}
