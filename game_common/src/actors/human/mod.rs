pub mod arm;
mod hand;
pub mod head;
pub mod leg;
pub mod torso;

use std::f32::consts::PI;

use bevy_asset::AssetServer;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::EntityCommands;
use bevy_hierarchy::BuildChildren;
use bevy_transform::prelude::Transform;
use glam::{Quat, Vec3};

use crate::components::actor::{ActorModel, Limb};
use crate::components::animation::{Bone, Skeleton};

use self::arm::{LowerLeftArm, LowerRightArm, UpperLeftArm, UpperRightArm};
use self::hand::RightHandItem;
use self::head::Head;
use self::leg::{LowerLeftLeg, LowerRightLeg, UpperLeftLeg, UpperRightLeg};
use self::torso::Torso;

const LIMB_HEAD: Limb = Limb::new(1);
const LIMB_TORSO: Limb = Limb::new(2);

const LIMB_UPPER_ARM_LEFT: Limb = Limb::new(3);
const LIMB_LOWER_ARM_LEFT: Limb = Limb::new(4);
const LIMB_UPPER_ARM_RIGHT: Limb = Limb::new(5);
const LIMB_LOWER_ARM_RIGHT: Limb = Limb::new(6);

const LIMB_UPPER_LEG_LEFT: Limb = Limb::new(7);
const LIMB_LOWER_LEG_LEFT: Limb = Limb::new(8);
const LIMB_UPPER_LEG_RIGHT: Limb = Limb::new(9);
const LIMB_LOWER_LEG_RIGHT: Limb = Limb::new(10);

#[derive(Clone, Debug, Default)]
pub struct Human {
    figure: HumanFigure,
}

impl Human {
    pub fn new(figure: HumanFigure) -> Self {
        Self { figure }
    }

    pub fn spawn(&self, assets: &AssetServer, commands: &mut EntityCommands<'_, '_, '_>) {
        let template = HumanTemplate {
            actor: commands.id(),
        };

        let mut entities = Vec::new();

        let mut skeleton = Skeleton {
            root: Entity::from_bits(0),
        };

        commands.add_children(|cmd| {
            // for entity in [
            //     cmd.spawn(Head::new(assets, &template)).id(),
            //     cmd.spawn(Torso::new(assets, &template)).id(),
            //     cmd.spawn(UpperLeftArm::new(assets, &template)).id(),
            //     cmd.spawn(LowerLeftArm::new(assets, &template)).id(),
            //     cmd.spawn(UpperRightArm::new(assets, &template)).id(),
            //     cmd.spawn(LowerRightArm::new(assets, &template)).id(),
            //     cmd.spawn(UpperLeftLeg::new(assets, &template)).id(),
            //     cmd.spawn(LowerLeftLeg::new(assets, &template)).id(),
            //     cmd.spawn(UpperRightLeg::new(assets, &template)).id(),
            //     cmd.spawn(LowerRightLeg::new(assets, &template)).id(),
            // ] {
            //     entities.push(entity);
            // }

            let head = Head::new(assets, &template);
            let torso = Torso::new(assets, &template);
            let upper_left_arm = UpperLeftArm::new(assets, &template);
            let lower_left_arm = LowerLeftArm::new(assets, &template);
            let upper_right_arm = UpperRightArm::new(assets, &template);
            let lower_right_arm = LowerRightArm::new(assets, &template);
            let upper_left_leg = UpperLeftLeg::new(assets, &template);
            let lower_left_leg = LowerLeftLeg::new(assets, &template);
            let upper_right_leg = UpperRightLeg::new(assets, &template);
            let lower_right_leg = LowerRightLeg::new(assets, &template);

            let right_arm_item = RightHandItem::new(assets, &template);

            let head = cmd
                .spawn(head)
                .insert(Bone {
                    children: vec![].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(-0.000266, 1.62081, 0.050296)
                            - Vec3::new(0.001521, 1.22746, -0.020366),
                        ..Default::default()
                    },
                })
                .id();
            let lla = cmd
                .spawn(lower_left_arm)
                .insert(Bone {
                    children: vec![].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.0, -0.5, 0.0),
                        ..Default::default()
                    },
                })
                .id();
            let ula = cmd
                .spawn(upper_left_arm)
                .insert(Bone {
                    children: vec![lla].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.234187, 1.2659, -0.064555)
                            - Vec3::new(0.001521, 1.22746, -0.020366),
                        ..Default::default()
                    },
                })
                .id();

            let rai = cmd
                .spawn(right_arm_item)
                .insert(Bone {
                    children: vec![].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.1, 0.5, 0.5),
                        ..Default::default()
                    },
                })
                .id();

            let lra = cmd
                .spawn(lower_right_arm)
                .insert(Bone {
                    children: vec![rai].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.0, -0.5, 0.0),
                        ..Default::default()
                    },
                })
                .id();
            let ura = cmd
                .spawn(upper_right_arm)
                .insert(Bone {
                    children: vec![lra].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(-0.228669, 1.26698, -0.059613)
                            - Vec3::new(0.001521, 1.22746, -0.020366),
                        ..Default::default()
                    },
                })
                .id();

            let lll = cmd
                .spawn(lower_left_leg)
                .insert(Bone {
                    children: vec![].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.0, -0.4, 0.0),
                        ..Default::default()
                    },
                })
                .id();
            let ull = cmd
                .spawn(upper_left_leg)
                .insert(Bone {
                    children: vec![lll].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.095273, 0.689199, 0.001766)
                            - Vec3::new(0.001521, 1.22746, -0.020366),
                        ..Default::default()
                    },
                })
                .id();

            let lrl = cmd
                .spawn(lower_right_leg)
                .insert(Bone {
                    children: vec![].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.0, -0.4, 0.0),
                        ..Default::default()
                    },
                })
                .id();
            let url = cmd
                .spawn(upper_right_leg)
                .insert(Bone {
                    children: vec![lrl].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(-0.092323, 0.648555, 0.009911)
                            - Vec3::new(0.001521, 1.22746, -0.020366),
                        ..Default::default()
                    },
                })
                .id();

            let torso = cmd
                .spawn(torso)
                .insert(Bone {
                    children: vec![head, ula, ura, ull, url].into_boxed_slice(),
                    offset: Transform {
                        translation: Vec3::new(0.001521, 1.22746, -0.020366),
                        rotation: Quat::from_axis_angle(Vec3::Y, PI),
                        ..Default::default()
                    },
                })
                .id();

            skeleton = Skeleton { root: torso };
        });

        commands.insert(ActorModel {
            entities: entities.into_boxed_slice(),
        });
        commands.insert(skeleton);
    }
}

#[derive(Clone, Debug, Default)]
pub struct HumanFigure {}

struct HumanTemplate {
    actor: Entity,
}
