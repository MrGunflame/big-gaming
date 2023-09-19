use game_asset::Asset;

use crate::color::Color;
use crate::texture::ImageId;

pub mod material;
pub mod mesh;

#[derive(Copy, Clone, Debug)]
pub struct PbrMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<ImageId>,

    pub normal_texture: Option<ImageId>,

    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<ImageId>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::default(),
            base_color: Color::WHITE,
            base_color_texture: None,
            normal_texture: None,
            roughness: 0.5,
            metallic: 0.0,
            metallic_roughness_texture: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

impl Asset for PbrMaterial {}
