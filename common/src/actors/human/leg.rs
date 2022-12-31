use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;

use crate::components::actor::ActorLimb;

use super::{
    HumanTemplate, LIMB_LOWER_LEG_LEFT, LIMB_LOWER_LEG_RIGHT, LIMB_UPPER_LEG_LEFT,
    LIMB_UPPER_LEG_RIGHT,
};

#[derive(Bundle)]
pub struct UpperLeftLeg {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl UpperLeftLeg {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_UPPER_LEG_LEFT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}

#[derive(Bundle)]
pub struct LowerLeftLeg {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl LowerLeftLeg {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_LOWER_LEG_LEFT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}

#[derive(Bundle)]
pub struct UpperRightLeg {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl UpperRightLeg {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_UPPER_LEG_RIGHT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}

#[derive(Bundle)]
pub struct LowerRightLeg {
    #[bundle]
    scene: SceneBundle,
    collider: Collider,
    actor_limb: ActorLimb,
}

impl LowerRightLeg {
    pub(super) fn new(assets: &AssetServer, template: &HumanTemplate) -> Self {
        let scene = SceneBundle {
            scene: assets.load("person.glb#Scene0"),
            ..Default::default()
        };
        let collider = Collider::cuboid(1.0, 1.0, 1.0);
        let actor_limb = ActorLimb {
            actor: template.actor,
            limb: LIMB_LOWER_LEG_RIGHT,
        };

        Self {
            scene,
            collider,
            actor_limb,
        }
    }
}
