use bevy::{
    math::Vec3,
    pbr::PbrBundle,
    prelude::{Bundle, Camera3d, Camera3dBundle, Component, Transform, *},
};
use bevy_rapier3d::prelude::*;

use crate::{Position, Rotation};

#[derive(Component)]
pub struct PlayerCharacter;

#[derive(Bundle)]
pub struct PlayerCharacterBundle {
    #[bundle]
    pub pbr: PbrBundle,
    pub velocity: Velocity,
    pub player_character: PlayerCharacter,
    pub gravity_scale: GravityScale,
    pub ccd: Ccd,
    pub collider: Collider,
    pub rigid_body: RigidBody,

    /// Lock rotation to prevent tilting the player character.
    pub locked_axes: LockedAxes,
    pub rotation: Rotation,
}

impl PlayerCharacterBundle {
    pub fn new(
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) -> Self {
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
                transform: Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::X, Vec3::Y),
                ..Default::default()
            },
            velocity: Velocity {
                linvel: Vec3::new(0.0, 0.0, 0.0),
                angvel: Vec3::new(0.0, 0.0, 0.0),
            },
            player_character: PlayerCharacter,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            rotation: Rotation::new(),
            gravity_scale: GravityScale(1.0),
            ccd: Ccd::enabled(),
            collider: Collider::cuboid(1.0, 1.0, 1.0),
            rigid_body: RigidBody::Dynamic,
        }
    }
}

#[derive(Bundle)]
pub struct PlayerCameraBundle {
    #[bundle]
    pub camera: Camera3dBundle,
    pub rotation: Rotation,
    pub camera_position: CameraPosition,
}

impl PlayerCameraBundle {
    pub fn new() -> Self {
        Self {
            camera: Camera3dBundle {
                transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::ZERO),
                ..Default::default()
            },
            rotation: Rotation::new(),
            camera_position: CameraPosition::FirstPerson,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Component)]
pub enum CameraPosition {
    #[default]
    FirstPerson,
    ThirdPerson,
}
