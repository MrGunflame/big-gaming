use bevy_ecs::bundle::Bundle;
use bevy_rapier3d::prelude::{
    CharacterAutostep, CharacterLength, KinematicCharacterController, LockedAxes,
};
use glam::Vec3;

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
    pub character_controller: KinematicCharacterController,
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
            character_controller: KinematicCharacterController {
                offset: CharacterLength::Absolute(0.01),
                up: Vec3::Y,
                max_slope_climb_angle: 45.0f32.to_radians(),
                min_slope_slide_angle: 30.0f32.to_radians(),
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.5),
                    min_width: CharacterLength::Absolute(0.2),
                    include_dynamic_bodies: true,
                }),
                snap_to_ground: Some(CharacterLength::Relative(0.2)),
                ..Default::default()
            },
        }
    }
}
