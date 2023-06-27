use bevy_ecs::bundle::Bundle;

use crate::components::actor::{Actor, ActorFigure, ActorFlags, ActorProperties, MovementSpeed};
use crate::components::inventory::Inventory;

use super::combat::CombatBundle;
use super::transform::TransformBundle;

#[derive(Clone, Debug, Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub transform: TransformBundle,
    #[bundle]
    pub combat: CombatBundle,
    pub actor: Actor,

    pub inventory: Inventory,
    pub actor_flags: ActorFlags,
    pub movement_speed: MovementSpeed,
    pub actor_figure: ActorFigure,

    pub properties: ActorProperties,
}

impl Default for ActorBundle {
    fn default() -> Self {
        Self {
            transform: TransformBundle::default(),
            combat: CombatBundle::default(),
            actor: Actor,
            inventory: Inventory::new(),
            actor_flags: ActorFlags::default(),
            movement_speed: MovementSpeed::default(),
            actor_figure: ActorFigure::default(),
            properties: ActorProperties::default(),
        }
    }
}
