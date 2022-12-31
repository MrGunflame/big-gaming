use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;

use crate::components::actor::ActorLimb;

use super::{
    HumanTemplate, LIMB_LOWER_ARM_LEFT, LIMB_LOWER_ARM_RIGHT, LIMB_UPPER_ARM_LEFT,
    LIMB_UPPER_ARM_RIGHT,
};

#[derive(Bundle)]
pub struct UpperLeftArm {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl UpperLeftArm {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_UPPER_ARM_LEFT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}

#[derive(Bundle)]
pub struct LowerLeftArm {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl LowerLeftArm {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_LOWER_ARM_LEFT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}

#[derive(Bundle)]
pub struct UpperRightArm {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl UpperRightArm {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_UPPER_ARM_RIGHT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}

#[derive(Bundle)]
pub struct LowerRightArm {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl LowerRightArm {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_LOWER_ARM_RIGHT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}
