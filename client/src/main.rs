#![feature(const_option)]
#![deny(unsafe_op_in_unsafe_fn)]

mod assets;
mod bundles;
mod components;
mod entities;
mod log;
mod plugins;
mod prev_transform;
mod systems;
mod ui;
mod utils;
mod window;
mod world;

use bevy::audio::AudioPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::gltf::GltfPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::scene::ScenePlugin;
use bevy::sprite::SpritePlugin;
use bevy::text::TextPlugin;
use bevy::time::TimePlugin;
use bevy::winit::WinitPlugin;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use bundles::ObjectBundle;
use common::actors::human::Human;
use common::archive::GameArchive;
use common::components::interaction::InteractionQueue;
use common::components::items::{Item, ItemId};
use components::Rotation;
use entities::actor::ActorBundle;
use entities::item::ItemBundle;
use entities::player::{PlayerCameraBundle, PlayerCharacterBundle};
use plugins::combat::CombatPlugin;
use plugins::interactions::InteractionsPlugin;
use plugins::respawn::RespawnPlugin;
use plugins::{CameraPlugin, HotkeyPlugin, MovementPlugin, ProjectilePlugin};
use ui::UiPlugin;
use world::chunk::ChunkPlugin;

fn main() {
    // log::Logger::new().init();

    let archive = GameArchive::new();
    archive.load("../core/data/items.json");

    App::new()
        .add_plugin(LogPlugin::default())
        .insert_resource(archive)
        .insert_resource(Msaa { samples: 4 })
        .add_plugin(CorePlugin::default())
        .add_plugin(TimePlugin)
        .add_plugin(TransformPlugin)
        .add_plugin(HierarchyPlugin)
        .add_plugin(DiagnosticsPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(WindowPlugin::default())
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin)
        .add_plugin(RenderPlugin)
        .add_plugin(ImagePlugin::default())
        .add_plugin(CorePipelinePlugin::default())
        .add_plugin(PbrPlugin)
        .add_plugin(SpritePlugin)
        .add_plugin(TextPlugin)
        .add_plugin(bevy::ui::UiPlugin)
        .add_plugin(AudioPlugin)
        .add_plugin(GilrsPlugin)
        .add_plugin(GltfPlugin)
        .add_plugin(WinitPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup)
        .add_plugin(CameraPlugin)
        .add_plugin(ProjectilePlugin)
        .add_plugin(CombatPlugin)
        .add_plugin(UiPlugin)
        .add_plugin(HotkeyPlugin)
        .add_plugin(MovementPlugin)
        .add_plugin(RespawnPlugin)
        .add_plugin(ChunkPlugin)
        .add_system_to_stage(CoreStage::Update, prev_transform::update_previous_transform)
        .add_plugin(InteractionsPlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
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

    commands.spawn(ObjectBundle::new(&asset_server));

    // THE BALL
    // commands
    //     .spawn(PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Icosphere {
    //             radius: 5.0,
    //             subdivisions: 69,
    //         })),
    //         material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
    //         transform: Transform::from_xyz(0.0, 20.0, 0.0),
    //         ..Default::default()
    //     })
    //     .insert(RigidBody::Dynamic)
    //     .insert(bevy_rapier3d::prelude::Velocity {
    //         linvel: Vec3::new(0.0, 0.0, 0.0),
    //         angvel: Vec3::new(0.2, 0.0, 0.0),
    //     })
    //     .insert(GravityScale(2.0))
    //     .insert(Sleeping::disabled())
    //     .insert(Ccd::enabled())
    //     .insert(Collider::ball(5.0))
    //     .insert(Restitution::coefficient(0.7))
    //     .insert(ColliderMassProperties::Density(69.0));

    // THE GAMER
    // commands
    //     .spawn(PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Box {
    //             min_x: 0.0,
    //             max_x: 1.0,
    //             min_y: 0.0,
    //             max_y: 2.0,
    //             min_z: 0.0,
    //             max_z: 1.0,
    //         })),
    //         material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
    //         ..Default::default()
    //     })
    //     .insert(RigidBody::Dynamic)
    //     .insert(GravityScale(1.0))
    //     .insert(Sleeping::disabled())
    //     .insert(Ccd::enabled())
    //     .insert(Collider::cuboid(1.0, 2.0, 1.0))
    //     .insert(Restitution::coefficient(0.7))
    //     .insert(ColliderMassProperties::Density(100.0));

    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Quad {
    //         size: Vec2::new(5.0, 1.0),
    //         flip: false,
    //     })),
    //     material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
    //     ..Default::default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Box {
    //         min_x: 0.0,
    //         max_x: 1.0,
    //         min_y: 0.0,
    //         max_y: 1.0,
    //         min_z: 0.0,
    //         max_z: 1.0,
    //     })),
    //     material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
    //     ..Default::default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Quad {
    //         size: Vec2::new(5.0, 1.0),
    //         flip: false,
    //     })),
    //     material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
    //     ..Default::default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Box {
    //         min_x: -100.0,
    //         max_x: 100.0,
    //         min_y: -0.1,
    //         max_y: 0.1,
    //         min_z: -0.1,
    //         max_z: 0.1,
    //     })),
    //     material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
    //     ..Default::default()
    // });

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
                // let scene = asset_server.load("thing2.glb#Scene0");

                // let collider = AsyncSceneCollider {
                //     handle: scene.clone_weak(),
                //     shape: Some(ComputedColliderShape::TriMesh),
                //     named_shapes: default(),
                // };

                let collider = Collider::cuboid(1.0, 1.0, 1.0);

                // let collider =
                //     Collider::from_bevy_mesh(&scene, &ComputedColliderShape::TriMesh).unwrap();

                // commands
                //     .spawn(SceneBundle {
                //         scene,
                //         transform: Transform::from_xyz(10.0, 5.0, 10.0),
                //         ..default()
                //     })
                //     .insert(RigidBody::Dynamic)
                //     .insert(GravityScale(1.0))
                //     .insert(Sleeping::disabled())
                //     .insert(Ccd::enabled())
                //     .insert(collider)
                //     // .insert(Collider::cuboid(1.0, 1.0, 1.0))
                //     .insert(Restitution::coefficient(0.7))
                //     .insert(ColliderMassProperties::Density(1.0));

                println!("spawned {x} {y} {z}");
            }
        }
    }

    let scene = asset_server.load("wall_1x5x3.glb#Scene0");
    let collider = Collider::cuboid(1.0, 5.0, 3.0);

    commands
        .spawn(SceneBundle {
            scene,
            transform: Transform::from_xyz(-10.0, 0.0, 10.0),
            ..default()
        })
        .insert(RigidBody::Fixed)
        .insert(collider);

    commands.spawn(ActorBundle::new(&asset_server));

    commands.spawn(PlayerCameraBundle::new());

    commands.spawn(ItemBundle::new(
        &asset_server,
        Item {
            id: ItemId(0.into()),
            components: None,
            resistances: None,
            ammo: None,
            damage: None,
            magazine: None,
            mass: Default::default(),
        },
    ));

    let mut cmd = commands.spawn(PlayerCharacterBundle::new(&asset_server));
    Human::default().spawn(&asset_server, &mut cmd);
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
