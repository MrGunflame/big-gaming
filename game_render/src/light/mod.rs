use bevy_ecs::prelude::{Bundle, Component};
use game_common::bundles::TransformBundle;

use crate::color::Color;

#[derive(Copy, Clone, Debug, Component)]
pub struct DirectionalLight {
    pub color: [f32; 3],
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
}

#[derive(Clone, Debug, Bundle)]
pub struct PointLightBundle {
    pub light: PointLight,
    #[bundle]
    pub transform: TransformBundle,
}
