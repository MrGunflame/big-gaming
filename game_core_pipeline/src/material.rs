use game_common::components::Color;

use crate::entities::ImageHandle;

#[derive(Clone, Debug)]
pub struct StandardMaterial {
    pub alpha_mode: AlphaMode,
    /// The color of the surface before lighting is applied.
    ///
    /// Defaults to [`WHITE`].
    ///
    /// [`WHITE`]: Color::WHITE
    pub base_color: Color,
    /// Texture for the `base_color`.
    pub base_color_texture: Option<ImageHandle>,
    pub normal_texture: Option<ImageHandle>,
    /// Flip the `Y` channel of the normal map to restore a DirectX normal map.
    ///
    /// Defaults to `false`.
    pub flip_normal_y: bool,
    /// Use two-channel normal map encoding.
    ///
    /// The X and Y channels are expected to be in the R and G channels respectively. The Z
    /// channel is reconstructed.
    ///
    /// Defaults to `false`.
    pub two_component_normal_encoding: bool,
    /// Linear perceptual roughness.
    ///
    /// Defaults to `0.5`.
    pub roughness: f32,
    /// How metallic the material is.
    ///
    /// Defaults to `0.0`.
    pub metallic: f32,
    /// Defaults to `0.5`.
    pub reflectance: f32,
    /// Texture for `roughness` and `metallic` factors.
    ///
    /// - Roughness in green channel.
    /// - Metallic in blue channel.
    pub metallic_roughness_texture: Option<ImageHandle>,
    /// Modifies the color of specular reflections.
    ///
    /// Defaults to [`WHITE`].
    ///
    /// [`WHITE`]: Color::WHITE
    pub specular_color: Color,
    /// Modifies the strength of specular reflections.
    ///
    /// A factor of `0.0` disables specular reflections completely.
    ///
    /// Defaults to `1.0`.
    pub specular_strength: f32,
    /// Texture for `specular_strength` and glossiness (`1.0 - roughness`).
    ///
    /// Note that if `metallic_roughess_texture` is set the glossiness is ignored and metallic
    /// sampled from the metallic-roughness texture instead of being converted from the specular
    /// channel.
    ///
    /// - Specular strength in the red channel.
    /// - Glossiness in the green channel.
    pub specular_glossiness_texture: Option<ImageHandle>,
    /// Whether to apply lighing on this material.
    ///
    /// Defaults to `false`.
    pub unlit: bool,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::default(),
            base_color: Color::WHITE,
            base_color_texture: None,
            normal_texture: None,
            flip_normal_y: false,
            two_component_normal_encoding: false,
            roughness: 0.5,
            metallic: 0.0,
            reflectance: 0.5,
            metallic_roughness_texture: None,
            specular_color: Color::WHITE,
            specular_strength: 1.0,
            specular_glossiness_texture: None,
            unlit: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
}
