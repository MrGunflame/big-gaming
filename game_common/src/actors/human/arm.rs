use bevy_asset::AssetServer;
use bevy_ecs::prelude::Bundle;
use bevy_rapier3d::prelude::Collider;
use bevy_scene::SceneBundle;
use bevy_transform::prelude::Transform;
use glam::Vec3;

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
            scene: assets.load("actor/human/upper_left_arm.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(0.234187, 1.2659, -0.064555),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.15, 0.1);
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
            scene: assets.load("actor/human/lower_left_arm.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(0.29221, 0.781748, 0.020128),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.3, 0.1);
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
            scene: assets.load("actor/human/upper_right_arm.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(-0.228669, 1.26698, -0.059613),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.15, 0.1);
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
            scene: assets.load("actor/human/lower_right_arm.glb#Scene0"),
            transform: Transform {
                translation: Vec3::new(-0.282336, 0.780438, 0.022348),
                ..Default::default()
            },
            ..Default::default()
        };
        let collider = Collider::cuboid(0.1, 0.3, 0.1);
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
