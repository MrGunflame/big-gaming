mod events;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Component;
use bevy_ecs::query::{With, Without};
use bevy_ecs::schedule::IntoSystemConfig;
use bevy_ecs::system::{Commands, Query, Res};
use game_common::bundles::TransformBundle;
use game_common::components::actor::ActorProperties;
use game_common::components::camera::CameraMode;
use game_common::components::player::HostPlayer;
use game_common::components::transform::Transform;
use game_render::camera::{Camera, CameraBundle, Projection, RenderTarget};
use game_render::light::{DirectionalLight, DirectionalLightBundle};
use glam::{Mat3, Quat, Vec3};

use crate::window::PrimaryWindow;

use super::movement::MovementSet;

/// The camera controlled by the hosting player.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct PrimaryCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_camera)
            .add_startup_system(events::register_events)
            .add_system(events::toggle_camera_position)
            .add_system(super::movement::camera_rotation)
            .add_system(super::movement::keyboard_input);
        // .add_system(synchronize_player_camera.after(MovementSet::Apply));
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
        .insert(CameraMode::FirstPerson)
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

fn synchronize_player_camera(
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

            //camera.look_at(player.translation, Vec3::Y);
        }
    }
}
