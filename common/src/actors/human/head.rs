use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;

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
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
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
