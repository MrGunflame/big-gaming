#![feature(const_trait_impl)]
#![feature(const_option)]
#![deny(unsafe_op_in_unsafe_fn)]

mod assets;
mod bundles;
mod components;
mod entities;
mod log;
mod net;
mod plugins;
mod prev_transform;
mod scene;
mod sky;
mod systems;
mod utils;
// mod window;
mod world;

use bevy::a11y::AccessibilityPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::sprite::SpritePlugin;
use bevy::text::TextPlugin;
use bevy::winit::WinitPlugin;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use clap::Parser;
use components::Rotation;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_common::scene::SceneTransition;
use game_core::CorePlugins;
use game_ui::UiPlugin;
use net::NetPlugin;
use plugins::interactions::InteractionsPlugin;
use plugins::{CameraPlugin, MovementPlugin};

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connect: Option<String>,
}

fn main() {
    let args = Args::parse();

    let archive = GameArchive::new();

    let loader = ModuleLoader::new(&archive);
    loader.load("../mods/core").unwrap();

    let mut app = App::new();
    app.insert_resource(archive);
    app.insert_resource(Msaa::Sample4);
    app.add_plugin(CorePlugins);
    // .add_plugin(TimePlugin)
    // .add_plugin(TransformPlugin)
    // .add_plugin(HierarchyPlugin)
    // .add_plugin(DiagnosticsPlugin)
    // .add_plugin(InputPlugin)
    app.add_plugin(WindowPlugin::default());
    // .add_plugin(AssetPlugin::default())
    // .add_plugin(ScenePlugin)
    app.add_plugin(RenderPlugin::default());
    app.add_plugin(ImagePlugin::default());
    app.add_plugin(CorePipelinePlugin::default());
    app.add_plugin(PbrPlugin::default());
    app.add_plugin(SpritePlugin);
    app.add_plugin(TextPlugin);
    app.add_plugin(bevy::ui::UiPlugin);
    app.add_plugin(GilrsPlugin);
    // .add_plugin(GltfPlugin)
    app.add_plugin(AccessibilityPlugin);
    app.add_plugin(WinitPlugin);
    // .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    app.add_plugin(RapierDebugRenderPlugin::default());
    app.add_plugin(CameraPlugin);
    // .add_plugin(ProjectilePlugin)
    // .add_plugin(CombatPlugin)
    app.add_plugin(scene::ScenePlugin);
    app.add_plugin(UiPlugin);
    app.add_plugin(MovementPlugin);
    // .add_plugin(game_core::movement::MovementPlugin)
    // .add_plugin(RespawnPlugin)
    // .add_plugin(ChunkPlugin::new(ChunkRegistry::new()))
    // .add_plugin(game_core::world::TimePlugin::default())
    app.add_plugin(InteractionsPlugin);
    // .add_plugin(game_core::animation::AnimationPlugin)
    // .add_plugin(AiPlugin)
    // .add_plugin(SpawnPlugin)
    // // .add_plugin(crate::ui::UiPlugin)
    app.add_plugin(game_input::InputPlugin);
    app.add_plugin(sky::SkyPlugin);
    // .add_plugin(game_core::world::ObjectPlugin)
    // .add_plugin(crate::plugins::combat::CombatPlugin)
    // .add_plugin(AudioPlugin::new())
    // .add_plugin(LevelPlugin)
    app.add_plugin(game_core::debug::DebugPlugin);
    app.add_plugin(NetPlugin::default());
    app.add_startup_system(setup);
    app.add_system(crate::world::terrain::load_terrain_mesh);

    if let Some(addr) = args.connect {
        tracing::info!("Connecting to {}", addr);

        // Transition to server connection scene.
        app.world.send_event(SceneTransition {
            from: game_common::scene::Scene::Loading,
            to: game_common::scene::Scene::ServerConnect { addr },
        });
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // THE FLOOOR
    // commands
    //     .spawn(PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Plane { size: 1000.0 })),
    //         material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
    //         transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    //         ..Default::default()
    //     })
    //     .insert(RigidBody::Fixed)
    //     .insert(Collider::cuboid(1000.0, 0.1, 1000.0));

    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 0.5, 5.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 1.5, 5.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 2.5, 5.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 3.5, 5.0)));

    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 0.5, 6.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 1.5, 6.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 2.5, 6.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 3.5, 6.0)));

    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 0.5, 7.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 1.5, 7.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 2.5, 7.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(5.0, 3.5, 7.0)));

    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(-5.0, 0.5, -5.0)));
    // commands.spawn(ObjectBundle::new(&asset_server).at(Vec3::new(10.0, 0.5, 5.0)));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    });

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

    // X
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box {
            min_x: -100.0,
            max_x: 100.0,
            min_y: 0.9,
            max_y: 1.1,
            min_z: 0.9,
            max_z: 1.1,
        })),
        material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        ..Default::default()
    });

    // Y
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box {
            min_y: -100.0,
            max_y: 100.0,
            min_x: 0.9,
            max_x: 1.1,
            min_z: 0.9,
            max_z: 1.1,
        })),
        material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
        ..Default::default()
    });

    // Z
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box {
            min_z: -100.0,
            max_z: 100.0,
            min_y: 0.9,
            max_y: 1.1,
            min_x: 0.9,
            max_x: 1.1,
        })),
        material: materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
        ..Default::default()
    });

    // INITIATE THE WALL
    // for x in 0..1 {
    //     for y in 0..1 {
    //         for z in 0..1 {
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

    // let collider = Collider::cuboid(1.0, 1.0, 1.0);

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

    //             println!("spawned {x} {y} {z}");
    //         }
    //     }
    // }

    // let scene = asset_server.load("wall_1x5x3.glb#Scene0");
    // let collider = Collider::cuboid(1.0, 5.0, 3.0);

    // commands
    //     .spawn(SceneBundle {
    //         scene,
    //         transform: Transform::from_xyz(-10.0, 0.0, 10.0),
    //         ..default()
    //     })
    //     .insert(RigidBody::Fixed)
    //     .insert(collider);

    // commands.spawn(ActorBundle::new(&asset_server));

    // Terrain mesh

    // let mut uvs = Vec::new();

    // let noise = noise::Simplex::default();

    // let mut height_map = Heightmap::default();

    // for index in 0..size_x * size_y {
    //     let x = index % size_x;
    //     let y = index / size_x;

    //     let res = noise.get([x as f64 / 20.0, y as f64 / 20.0]);
    //     height_map.nodes.push(res as f32);
    // }

    // let chunk = TerrainMesh::new(height_map);
    // let mesh = chunk.mesh();
    // let collider = chunk.collider();

    // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // mesh.insert_attribute(
    //     Mesh::ATTRIBUTE_POSITION,
    //     vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
    // );
    // mesh.set_indices(Some(Indices::U32(vec![0, 1, 2])));

    // let img: Handle<Image> = asset_server.load("gw316.jpg");

    // commands
    //     .spawn(PbrBundle {
    //         mesh: meshes.add(mesh),
    //         material: materials.add(StandardMaterial {
    //             base_color: Color::RED,
    //             base_color_texture: Some(img),
    //             ..Default::default()
    //         }),
    //         transform: Transform::from_translation(Vec3::new(15.0, 10.0, 0.0)),
    //         ..default()
    //     })
    //     .insert(RigidBody::Fixed)
    //     .insert(collider);

    // commands.spawn(ItemBundle::new(
    //     &asset_server,
    //     Item {
    //         id: ItemId(0.into()),
    //         components: None,
    //         resistances: None,
    //         ammo: None,
    //         damage: None,
    //         magazine: None,
    //         mass: Default::default(),
    //         cooldown: Cooldown::new(Duration::ZERO),
    //     },
    // ));

    // let mut cmd = commands.spawn(PlayerCharacterBundle::new(&asset_server));
    // Human::default().spawn(&asset_server, &mut cmd);

    // let mut cmd = commands.spawn(ActorBundle::new(&asset_server));
    // Human::default().spawn(&asset_server, &mut cmd);
    // cmd.insert(AiBundle::default());
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
