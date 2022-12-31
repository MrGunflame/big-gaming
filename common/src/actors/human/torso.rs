use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;

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
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
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
