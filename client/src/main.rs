#![feature(const_option)]

mod assets;
mod components;
mod entities;
mod inventory;
mod plugins;
mod systems;
mod ui;
mod utils;
mod world;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use components::Rotation;
use entities::actor::ActorBundle;
use entities::player::{PlayerCameraBundle, PlayerCharacterBundle};
use plugins::combat::CombatPlugin;
use plugins::{CameraPlugin, HotkeyPlugin, MovementPlugin, ProjectilePlugin};
use ui::UiPlugin;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup)
        .add_plugin(CameraPlugin)
        .add_plugin(ProjectilePlugin)
        .add_plugin(CombatPlugin)
        .add_plugin(UiPlugin)
        .add_plugin(HotkeyPlugin)
        .add_plugin(MovementPlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_server: Res<AssetServer>,
) {
    // THE FLOOOR
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            ..Default::default()
        })
        .insert(RigidBody::Fixed)
        .insert(Collider::cuboid(1000.0, 0.1, 1000.0));

    // THE BALL
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 5.0,
                subdivisions: 69,
            })),
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            transform: Transform::from_xyz(0.0, 20.0, 0.0),
            ..Default::default()
        })
        .insert(RigidBody::Dynamic)
        .insert(bevy_rapier3d::prelude::Velocity {
            linvel: Vec3::new(0.0, 0.0, 0.0),
            angvel: Vec3::new(0.2, 0.0, 0.0),
        })
        .insert(GravityScale(2.0))
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(Collider::ball(5.0))
        .insert(Restitution::coefficient(0.7))
        .insert(ColliderMassProperties::Density(69.0));

    // THE GAMER
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box {
                min_x: 0.0,
                max_x: 1.0,
                min_y: 0.0,
                max_y: 2.0,
                min_z: 0.0,
                max_z: 1.0,
            })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            ..Default::default()
        })
        .insert(RigidBody::Dynamic)
        .insert(GravityScale(1.0))
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(Collider::cuboid(1.0, 2.0, 1.0))
        .insert(Restitution::coefficient(0.7))
        .insert(ColliderMassProperties::Density(100.0));

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad {
            size: Vec2::new(5.0, 1.0),
            flip: false,
        })),
        material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box {
            min_x: 0.0,
            max_x: 1.0,
            min_y: 0.0,
            max_y: 1.0,
            min_z: 0.0,
            max_z: 1.0,
        })),
        material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad {
            size: Vec2::new(5.0, 1.0),
            flip: false,
        })),
        material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box {
            min_x: -100.0,
            max_x: 100.0,
            min_y: -0.1,
            max_y: 0.1,
            min_z: -0.1,
            max_z: 0.1,
        })),
        material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        ..Default::default()
    });

    // INITIATE THE WALL
    for x in 0..1 {
        for y in 0..1 {
            for z in 0..1 {
                // commands
                //     .spawn_bundle(PbrBundle {
                //         mesh: meshes.add(Mesh::from(shape::Box {
                //             min_x: 0.0,
                //             max_x: 1.0,
                //             min_y: 0.0,
                //             max_y: 1.0,
                //             min_z: 0.0,
                //             max_z: 1.0,
                //         })),
                //         material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                //         transform: Transform::from_xyz(
                //             10.0 + x as f32,
                //             10.0 + y as f32,
                //             10.0 + z as f32,
                //         ),
                //         ..Default::default()
                //     })
                //     .insert(RigidBody::Dynamic)
                //     .insert(GravityScale(1.0))
                //     .insert(Sleeping::disabled())
                //     .insert(Ccd::enabled())
                //     .insert(Collider::cuboid(1.0, 1.0, 1.0))
                //     .insert(Restitution::coefficient(0.7))
                //     .insert(ColliderMassProperties::Density(1.0));

                // let scene = asset_server.load("WaterBottle.gltf#Scene0");
                let scene = asset_server.load("thing2.glb#Scene0");

                // let collider = AsyncSceneCollider {
                //     handle: scene.clone_weak(),
                //     shape: Some(ComputedColliderShape::TriMesh),
                //     named_shapes: default(),
                // };

                let collider = Collider::cuboid(1.0, 1.0, 1.0);

                // let collider =
                //     Collider::from_bevy_mesh(&scene, &ComputedColliderShape::TriMesh).unwrap();

                commands
                    .spawn(SceneBundle {
                        scene,
                        transform: Transform::from_xyz(10.0, 5.0, 10.0),
                        ..default()
                    })
                    .insert(RigidBody::Dynamic)
                    .insert(GravityScale(1.0))
                    .insert(Sleeping::disabled())
                    .insert(Ccd::enabled())
                    .insert(collider)
                    // .insert(Collider::cuboid(1.0, 1.0, 1.0))
                    .insert(Restitution::coefficient(0.7))
                    .insert(ColliderMassProperties::Density(1.0));

                println!("spawned {x} {y} {z}");
            }
        }
    }

    commands.spawn(ActorBundle::new(&asset_server));

    commands.spawn(PlayerCameraBundle::new());
    commands.spawn(PlayerCharacterBundle::new(&asset_server));
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Bundle)]
pub struct PlayerCamera {
    #[bundle]
    camera: Camera3dBundle,
    rotation: Rotation,
    velocity: Velocity,
}
