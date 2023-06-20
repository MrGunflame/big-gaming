use game_render::color::Color;
use game_render::pbr::AlphaMode;

#[derive(Clone, Debug)]
pub struct GltfMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<Vec<u8>>,
    pub normal_texture: Option<Vec<u8>>,
    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<Vec<u8>>,
}
