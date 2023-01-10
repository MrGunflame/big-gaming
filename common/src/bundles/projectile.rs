use bevy_ecs::bundle::Bundle;
use bevy_scene::SceneBundle;

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
