use game_asset::Asset;
use game_common::components::rendering::Color;

use crate::texture::ImageId;

pub mod material;
pub mod mesh;

#[derive(Copy, Clone, Debug)]
pub struct PbrMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<ImageId>,

    pub normal_texture: Option<ImageId>,

    /// Linear perceptual roughness.
    ///
    /// Defaults to `0.5`.
    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<ImageId>,

    /// Specular intensity between `[0.0, 1.0]`.
    ///
    /// Defaults to `0.5`, which corresponds to 0.04 in the shader.
    pub reflectance: f32,
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
            reflectance: 0.5,
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
