use bevy_ecs::prelude::{Bundle, Component};
use image::{ImageBuffer, Rgba};

use crate::mesh::Mesh;

#[derive(Clone, Debug, Bundle)]
pub struct MaterialMeshBundle {
    pub mesh: Mesh,
    pub material: Material,
}

#[derive(Clone, Debug, Component)]
pub struct Material {
    pub color: [f32; 4],
    pub color_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
}
