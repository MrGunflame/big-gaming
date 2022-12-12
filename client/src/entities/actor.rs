use bevy::prelude::{AssetServer, Bundle, Vec3};
use bevy::scene::SceneBundle;
use bevy_rapier3d::prelude::{
    AdditionalMassProperties, Ccd, Collider, LockedAxes, RigidBody, Velocity,
};

use crate::components::{Actor, ActorState, Rotation};
use crate::plugins::combat::{CombatBundle, Health, IncomingDamage};
use crate::plugins::movement::MovementSpeed;

#[derive(Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub scene: SceneBundle,
    #[bundle]
    pub combat: CombatBundle,

    pub rotation: Rotation,

    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub ccd: Ccd,
    pub collider: Collider,
    pub mass: AdditionalMassProperties,
    pub locked_axes: LockedAxes,

    pub actor: Actor,
    pub actor_state: ActorState,
    pub movement_speed: MovementSpeed,
}

impl ActorBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            scene: SceneBundle {
                scene: assets.load("person.glb#Scene0"),
                ..Default::default()
            },
            rotation: Rotation::new(),
            velocity: Velocity {
                linvel: Vec3::new(0.0, 0.0, 0.0),
                angvel: Vec3::new(0.0, 0.0, 0.0),
            },
            locked_axes: LockedAxes::ROTATION_LOCKED,
            ccd: Ccd::enabled(),
            collider: Collider::cuboid(1.0, 1.0, 1.0),
            rigid_body: RigidBody::Dynamic,
            mass: AdditionalMassProperties::Mass(100.0),
            actor: Actor,
            combat: CombatBundle::new(),
            actor_state: ActorState::DEAD,
            movement_speed: MovementSpeed(3.0),
        }
    }
}
