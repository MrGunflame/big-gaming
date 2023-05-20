pub struct PbrMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: [f32; 4],
    pub base_color_texture: Option<Vec<u8>>,

    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<Vec<u8>>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}
