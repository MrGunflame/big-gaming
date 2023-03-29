mod events;

use std::f32::consts::PI;

use bevy::prelude::{
    Camera3dBundle, Commands, IntoSystemConfig, Mat3, Plugin, Quat, Query, Res, Transform, Vec3,
    With, Without,
};
use bevy::time::Time;
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::camera::CameraMode;
use game_common::components::movement::Movement;
use game_common::components::player::HostPlayer;

use crate::components::settings::CameraSettings;

use super::movement::MovementSet;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup_camera)
            .add_startup_system(events::register_events)
            .add_system(events::toggle_camera_position)
            .insert_resource(CameraSettings::default())
            // .add_system(crate::systems::input::interact_target)
            .add_system(synchronize_player_camera.after(MovementSet::Apply))
            .add_system(head_bumping.after(synchronize_player_camera));
        // .add_system(toggle_camera_position)
        // .add_system(adjust_camera_distance);
    }
}

fn setup_camera(mut commands: Commands) {
    // Spawn the camera at any positon. It will be moved to the
    // correct location at the first call to synchronize_player_camera.
    commands
        .spawn(Camera3dBundle {
            transform: Transform {
                translation: Vec3::splat(0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(1.0),
            },
            ..Default::default()
        })
        .insert(CameraMode::FirstPerson);
}

fn synchronize_player_camera(
    settings: Res<CameraSettings>,
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

            camera.translation = player.translation
                + rotation_matrix
                    * Vec3::new(
                        settings.offset.x,
                        settings.offset.y,
                        settings.offset.z + *distance,
                    );

            camera.look_at(player.translation, Vec3::Y);
        }
    }
}

/// Apply a periodic head bumping effect while the player is moving.
fn head_bumping(
    time: Res<Time>,
    settings: Res<CameraSettings>,
    players: Query<(&Transform, &MovementSpeed), (With<HostPlayer>, With<Movement>)>,
    mut cameras: Query<(&mut Transform, &CameraMode), Without<HostPlayer>>,
) {
    // Only apply head bumping when the player is moving.
    let Ok((player, speed)) = players.get_single() else {
        return;
    };

    let (mut camera, position) = cameras.single_mut();

    if matches!(position, CameraMode::ThirdPerson { distance: _ }) {
        return;
    }

    // Relative distance between current and next frame.
    // let distance = player.translation.distance(movement.desination).abs();
    let distance = speed.0;

    // F
    let sc = time.elapsed_seconds() * PI * 2.0 * distance;
    let offset = sc.sin() * 0.05 * settings.head_bumping;

    camera.translation += Vec3::new(0.0, offset, 0.0);
}
