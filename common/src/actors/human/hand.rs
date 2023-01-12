use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_scene::SceneBundle;
use bevy_transform::components::Transform;
use glam::Vec3;

use super::HumanTemplate;

#[derive(Bundle)]
pub(super) struct RightHandItem {
    #[bundle]
    scene: SceneBundle,
}

impl RightHandItem {
    pub fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("pistol.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                ..Default::default()
            },
            ..Default::default()
        };

        Self { scene }
    }
}
