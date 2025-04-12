
fn compute_diffuse_color(base_color: vec4<f32>, metallic: f32) -> vec3<f32> {
    return base_color.rgb * (1.0 - metallic);
}

fn compute_f0(base_color: vec3<f32>, metallic: f32, reflectance: f32) -> vec3<f32> {
    return base_color * metallic + (reflectance * (1.0 - metallic));
}

fn compute_dielectic_f0(reflectance: f32) -> f32 {
    return 0.16 * reflectance * reflectance;
}

fn compute_metallic_from_specular_color(specular_color: vec3<f32>) -> f32 {
    return max(max(specular_color.r, specular_color.g), specular_color.b);
}

fn compute_roughness_from_glossiness(glossiness: f32) -> f32 {
    return 1.0 - glossiness;
}

fn perceptual_roughness_to_roughness(perceptual_roughness: f32) -> f32 {
    // We clamp the roughness to 0.089 to prevent precision problems.
    // We might be able to lower that value to 0.045 since we're using
    // 32-bit floats.
    // See https://google.github.io/filament/Filament.html#toc4.8.3.3
    let clamped_perceptual_roughness = clamp(perceptual_roughness, 0.089, 1.0);
    return clamped_perceptual_roughness * clamped_perceptual_roughness;
}
