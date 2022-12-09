use bevy::prelude::{
    shape, Assets, Bundle, Color, Mesh, PbrBundle, StandardMaterial, Transform, Vec3,
};
use bevy_rapier3d::prelude::{
    AdditionalMassProperties, Ccd, Collider, LockedAxes, RigidBody, Velocity,
};
use bevy_rapier3d::rapier::prelude::Shape;

use crate::components::{Actor, ActorState};
use crate::plugins::combat::{CombatBundle, Health, IncomingDamage};

#[derive(Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub pbr: PbrBundle,
    #[bundle]
    pub combat: CombatBundle,

    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub ccd: Ccd,
    pub collider: Collider,
    pub mass: AdditionalMassProperties,
    pub locked_axes: LockedAxes,

    pub actor: Actor,
    pub actor_state: ActorState,
}

impl ActorBundle {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            pbr: PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box {
                    min_x: 0.0,
                    max_x: 1.0,
                    min_y: 0.0,
                    max_y: 3.0,
                    min_z: 0.0,
                    max_z: 1.0,
                })),
                material: materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
                transform: Transform::from_xyz(10.0, 0.0, 10.0).looking_at(Vec3::X, Vec3::Y),
                ..Default::default()
            },
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
            actor_state: ActorState::Alive,
        }
    }
}
