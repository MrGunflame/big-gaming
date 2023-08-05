pub mod pipeline;

use bevy_ecs::prelude::{Bundle, Component};
use game_common::bundles::TransformBundle;

use crate::color::Color;

#[derive(Copy, Clone, Debug, Component)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
}

#[derive(Clone, Debug, Bundle)]
pub struct DirectionalLightBundle {
    pub light: DirectionalLight,
    #[bundle]
    pub transform: TransformBundle,
}

#[derive(Copy, Clone, Debug, Component)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Clone, Debug, Bundle)]
pub struct PointLightBundle {
    pub light: PointLight,
    #[bundle]
    pub transform: TransformBundle,
}

#[derive(Copy, Clone, Debug, Component)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    /// Inner cutoff angle
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

#[derive(Clone, Debug, Bundle)]
pub struct SpotLightBundle {
    pub light: SpotLight,
    #[bundle]
    pub transform: TransformBundle,
}
