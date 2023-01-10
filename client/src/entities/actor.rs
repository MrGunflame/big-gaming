use bevy::prelude::{AssetServer, Bundle, Component, Vec3};
use bevy::scene::SceneBundle;
use bevy::transform::TransformBundle;
use bevy_rapier3d::na::vector;
use bevy_rapier3d::prelude::{
    AdditionalMassProperties, AsyncCollider, Ccd, CharacterAutostep, CharacterLength, Collider,
    KinematicCharacterController, LockedAxes, RigidBody, Velocity,
};
use common::components::actor::{Actor, ActorState, MovementSpeed};
use common::components::animation::AnimationQueue;
use common::components::inventory::{Equipment, EquipmentSlot, Inventory};
use common::components::items::{Item, ItemId};

use crate::bundles::VisibilityBundle;
use crate::components::Rotation;
use crate::plugins::combat::CombatBundle;

#[derive(Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub transform: crate::bundles::TransformBundle,
    #[bundle]
    pub visibility: VisibilityBundle,
    // #[bundle]
    // pub scene: SceneBundle,
    #[bundle]
    pub combat: CombatBundle,

    pub rotation: Rotation,

    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub ccd: Ccd,
    // pub collider: Collider,
    // pub mass: AdditionalMassProperties,
    pub locked_axes: LockedAxes,

    pub actor: Actor,
    pub actor_state: ActorState,
    pub movement_speed: MovementSpeed,
    pub character_controller: KinematicCharacterController,
    pub actor_figure: ActorFigure,
    pub inventory: Inventory,
    pub equipment: Equipment,
    pub animation_queue: AnimationQueue,
}

impl ActorBundle {
    pub fn new(assets: &AssetServer) -> Self {
        let mut equipment = Equipment::new();
        equipment.insert(
            EquipmentSlot::MAIN_HAND,
            Item {
                id: ItemId(0.into()),
                resistances: None,
                damage: None,
                components: None,
                magazine: Some(31),
                mass: Default::default(),
                ammo: Some(ItemId(64.into())),
            },
        );

        Self {
            transform: crate::bundles::TransformBundle::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            // scene: SceneBundle {
            //     scene: assets.load("person.glb#Scene0"),
            //     ..Default::default()
            // },
            visibility: VisibilityBundle::new(),
            rotation: Rotation::new(),
            velocity: Velocity {
                linvel: Vec3::new(0.0, 0.0, 0.0),
                angvel: Vec3::new(0.0, 0.0, 0.0),
            },
            locked_axes: LockedAxes::ROTATION_LOCKED,
            ccd: Ccd::enabled(),
            // collider: Collider::cuboid(1.0, 0.5, 1.0),
            // collider,
            rigid_body: RigidBody::Dynamic,
            // mass: AdditionalMassProperties::Mass(100.0),
            actor: Actor,
            combat: CombatBundle::new(),
            actor_state: ActorState::DEFAULT,
            movement_speed: MovementSpeed(3.0),
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
            actor_figure: ActorFigure {
                eyes: Vec3::new(0.0, 1.65, -0.3),
            },
            inventory: Inventory::new(),
            equipment,
            animation_queue: AnimationQueue::new(),
        }
    }
}

#[derive(Clone, Debug, Component)]
pub struct ActorFigure {
    /// The offset to the eyes.
    ///
    /// This is where the first-person camera should be placed.
    pub eyes: Vec3,
}
