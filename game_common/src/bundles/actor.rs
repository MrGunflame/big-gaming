use bevy_ecs::bundle::Bundle;
use bevy_rapier3d::prelude::LockedAxes;

use crate::components::actor::{Actor, ActorFigure, ActorFlags, MovementSpeed};
use crate::components::inventory::{Equipment, Inventory};

use super::combat::CombatBundle;
use super::physics::DynamicPhysicsBundle;
use super::transform::TransformBundle;
use super::visibility::VisibilityBundle;

#[derive(Clone, Debug, Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub transform: TransformBundle,
    #[bundle]
    pub visibility: VisibilityBundle,
    #[bundle]
    pub combat: CombatBundle,
    #[bundle]
    pub physics: DynamicPhysicsBundle,
    pub actor: Actor,

    pub inventory: Inventory,
    pub equipment: Equipment,
    pub actor_flags: ActorFlags,
    pub movement_speed: MovementSpeed,
    pub actor_figure: ActorFigure,

    pub locked_axes: LockedAxes,
}

impl Default for ActorBundle {
    fn default() -> Self {
        Self {
            locked_axes: LockedAxes::ROTATION_LOCKED,
            transform: TransformBundle::default(),
            visibility: VisibilityBundle::default(),
            combat: CombatBundle::default(),
            physics: DynamicPhysicsBundle::default(),
            actor: Actor,
            inventory: Inventory::new(),
            equipment: Equipment::new(),
            actor_flags: ActorFlags::default(),
            movement_speed: MovementSpeed::default(),
            actor_figure: ActorFigure::default(),
        }
    }
}
