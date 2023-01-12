use std::f32::consts::PI;

use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;
use bevy_transform::prelude::Transform;
use glam::Vec3;

use super::{HumanTemplate, LIMB_TORSO};
use crate::components::actor::ActorLimb;

#[derive(Bundle)]
pub struct Torso {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl Torso {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("actor/human/torso.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(0.001521, 1.22746, -0.020366),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.2, 0.4, 0.2);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_TORSO,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}
