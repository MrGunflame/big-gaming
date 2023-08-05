use bevy_ecs::prelude::Bundle;
use game_asset::{Asset, Handle};
use game_common::bundles::TransformBundle;

use crate::color::Color;
use crate::mesh::Mesh;
use crate::texture::ImageHandle;

pub mod material;
pub mod mesh;

#[derive(Clone, Debug, Bundle)]
pub struct PbrBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<PbrMaterial>,
    #[bundle]
    pub transform: TransformBundle,
}

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<ImageHandle>,

    pub normal_texture: Option<ImageHandle>,

    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<ImageHandle>,
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
