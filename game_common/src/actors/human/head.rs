use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;
use bevy_transform::prelude::Transform;
use glam::Vec3;

use crate::components::actor::ActorLimb;

use super::{HumanTemplate, LIMB_HEAD};

#[derive(Bundle)]
pub struct Head {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl Head {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("actor/human/head.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(-0.000266, 1.62081, 0.050296),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.15, 0.1);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_HEAD,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}
