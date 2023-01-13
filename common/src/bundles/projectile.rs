use bevy_asset::AssetServer;
use bevy_ecs::bundle::Bundle;
use bevy_scene::SceneBundle;
use glam::Vec3;

use crate::components::projectile::Projectile;

use super::physics::DynamicPhysicsBundle;

#[derive(Default, Bundle)]
pub struct ProjectileBundle {
    #[bundle]
    pub scene: SceneBundle,
    #[bundle]
    pub physics: DynamicPhysicsBundle,

    pub projectile: Projectile,
}

impl ProjectileBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            scene: SceneBundle {
                scene: assets.load("bullet.glb#Scene0"),
                ..Default::default()
            },
            physics: DynamicPhysicsBundle::default(),
            projectile: Projectile,
        }
    }

    pub fn at(mut self, translation: Vec3) -> Self {
        self.scene.transform.translation = translation;
        self
    }

    pub fn looking_at(mut self, target: Vec3) -> Self {
        self.scene.transform.look_at(target, Vec3::Y);
        self
    }
}
