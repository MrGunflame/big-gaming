pub mod pipeline;

use game_common::components::{Color, Transform};

use crate::entities::SceneId;

#[derive(Copy, Clone, Debug)]
pub struct DirectionalLight {
    pub transform: Transform,
    pub scene: SceneId,
    pub color: Color,
    pub illuminance: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct PointLight {
    pub transform: Transform,
    pub scene: SceneId,
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct SpotLight {
    pub transform: Transform,
    pub scene: SceneId,
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    /// Inner cutoff angle
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}
