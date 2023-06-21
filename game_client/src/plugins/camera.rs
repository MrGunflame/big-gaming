mod events;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Component;
use bevy_ecs::query::{With, Without};
use bevy_ecs::schedule::IntoSystemConfig;
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_asset::Assets;
use game_common::bundles::TransformBundle;
use game_common::components::actor::ActorProperties;
use game_common::components::camera::CameraMode;
use game_common::components::player::HostPlayer;
use game_common::components::transform::Transform;
use game_render::camera::{Camera, CameraBundle, Projection, RenderTarget};
use game_render::color::Color;
use game_render::light::{DirectionalLight, DirectionalLightBundle};
use game_render::mesh::Mesh;
use game_render::pbr::{PbrBundle, PbrMaterial};
use game_render::shape;
use glam::{Mat3, Quat, Vec3};

use crate::window::PrimaryWindow;

/// The camera controlled by the hosting player.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct PrimaryCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(events::register_events);

        app.add_system(synchronize_player_camera);
        app.add_system(events::toggle_camera_position);

        app.add_startup_system(setup_camera);

        // Display coordinate cross for debugging
        app.add_startup_system(create_coordinate_axes);
        app.add_system(update_coordinate_axes);
    }
}

fn setup_camera(mut commands: Commands, target: Res<PrimaryWindow>) {
    // Spawn the camera at any positon. It will be moved to the
    // correct location at the first call to synchronize_player_camera.
    commands
        .spawn(CameraBundle {
            transform: Transform {
                translation: Vec3::splat(0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(1.0),
            },
            camera: Camera {
                projection: Projection::default(),
                target: RenderTarget::Window(target.0),
            },
        })
        .insert(CameraMode::Detached)
        .insert(PrimaryCamera);

    commands.spawn(DirectionalLightBundle {
        light: DirectionalLight {
            color: [1.0; 3],
            illuminance: 1.0,
        },
        transform: TransformBundle {
            transform: Transform {
                translation: Vec3::splat(-1.0),
                ..Default::default()
            },
            ..Default::default()
        },
    });
}

pub fn synchronize_player_camera(
    players: Query<(&Transform, &ActorProperties), With<HostPlayer>>,
    mut cameras: Query<(&mut Transform, &CameraMode), Without<HostPlayer>>,
) {
    let Ok((player, props)) = players.get_single() else {
        return;
    };
    let (mut camera, mode) = cameras.single_mut();

    match mode {
        CameraMode::FirstPerson => {
            let rotation_matrix = Mat3::from_quat(player.rotation);
            camera.translation = player.translation + rotation_matrix * props.eyes;

            camera.rotation = props.rotation;
        }
        CameraMode::ThirdPerson { distance } => {
            let rotation_matrix = Mat3::from_quat(player.rotation);

            camera.translation =
                player.translation + rotation_matrix * Vec3::new(0.0, 0.0, 0.0 + *distance);
        }
        // We don't want to sync in detached mode.
        CameraMode::Detached => (),
    }
}

#[derive(Copy, Clone, Debug, Component)]
struct AxisMarker;

fn create_coordinate_axes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PbrMaterial>>,
) {
    for (mesh, color) in [
        (
            shape::Box {
                min_x: 0.0,
                max_x: 1.0,
                min_y: -0.1,
                max_y: 0.1,
                min_z: -0.1,
                max_z: 0.1,
            },
            Color::RED,
        ),
        (
            shape::Box {
                min_x: -0.1,
                max_x: 0.1,
                min_y: 0.0,
                max_y: 1.0,
                min_z: -0.1,
                max_z: 0.1,
            },
            Color::GREEN,
        ),
        (
            shape::Box {
                min_x: -0.1,
                max_x: 0.1,
                min_y: -0.1,
                max_y: 0.1,
                min_z: 0.0,
                max_z: 1.0,
            },
            Color::BLUE,
        ),
    ] {
        commands
            .spawn(PbrBundle {
                mesh: meshes.insert(mesh.into()),
                material: materials.insert(PbrMaterial {
                    base_color: color,
                    ..Default::default()
                }),
                transform: TransformBundle::default(),
            })
            .insert(AxisMarker);
    }
}

fn update_coordinate_axes(
    cameras: Query<&Transform, With<PrimaryCamera>>,
    mut axes: Query<(&mut Transform, &AxisMarker), Without<PrimaryCamera>>,
) {
    let camera = cameras.single();

    for (mut transform, _) in &mut axes {
        transform.translation = camera.translation + ((camera.rotation * -Vec3::Z) * 10.0);
    }
}
