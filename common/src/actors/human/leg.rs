use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;
use bevy_transform::prelude::Transform;
use glam::Vec3;

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
            scene: assets.load("actor/human/upper_left_leg.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(0.095273, 0.689199, 0.001766),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.2, 0.1);
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
            scene: assets.load("actor/human/lower_left_leg.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(0.124872, 0.087127, 0.092178),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.3, 0.1);
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
            scene: assets.load("actor/human/upper_right_leg.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(-0.092323, 0.648555, 0.009911),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.2, 0.1);
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
            scene: assets.load("actor/human/lower_right_leg.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(-0.125781, 0.071524, 0.094625),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.3, 0.1);
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
