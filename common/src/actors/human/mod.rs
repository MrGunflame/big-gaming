pub mod arm;
pub mod head;
pub mod leg;
pub mod torso;

use bevy_asset::AssetServer;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::EntityCommands;
use bevy_hierarchy::BuildChildren;

use crate::components::actor::{ActorModel, Limb};

use self::arm::{LowerLeftArm, LowerRightArm, UpperLeftArm, UpperRightArm};
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

        commands.add_children(|cmd| {
            for entity in [
                cmd.spawn(Head::new(assets, &template)).id(),
                cmd.spawn(Torso::new(assets, &template)).id(),
                cmd.spawn(UpperLeftArm::new(assets, &template)).id(),
                cmd.spawn(LowerLeftArm::new(assets, &template)).id(),
                cmd.spawn(UpperRightArm::new(assets, &template)).id(),
                cmd.spawn(LowerRightArm::new(assets, &template)).id(),
                cmd.spawn(UpperLeftLeg::new(assets, &template)).id(),
                cmd.spawn(LowerLeftLeg::new(assets, &template)).id(),
                cmd.spawn(UpperRightLeg::new(assets, &template)).id(),
                cmd.spawn(LowerRightLeg::new(assets, &template)).id(),
            ] {
                entities.push(entity);
            }
        });

        commands.insert(ActorModel {
            entities: entities.into_boxed_slice(),
        });
    }
}

#[derive(Clone, Debug, Default)]
pub struct HumanFigure {}

struct HumanTemplate {
    actor: Entity,
}
