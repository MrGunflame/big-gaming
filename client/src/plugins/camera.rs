mod events;

use std::f32::consts::PI;

use bevy::prelude::{
    CoreStage, IntoSystemDescriptor, Mat3, Plugin, Query, Res, Transform, Vec3, With, Without,
};
use bevy::time::Time;
use common::components::actor::ActorFigure;
use common::components::movement::Movement;

use crate::components::settings::CameraSettings;
use crate::entities::player::{CameraPosition, PlayerCharacter};

use self::events::{adjust_camera_distance, register_events, toggle_camera_position};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events)
            .insert_resource(CameraSettings::default())
            .add_system(crate::systems::input::transform_system)
            .add_system(crate::systems::input::mouse_button_input)
            .add_system(crate::systems::input::interact_target)
            .add_system(synchronize_player_camera)
            .add_system(head_bumping.after(synchronize_player_camera))
            .add_system(toggle_camera_position)
            .add_system(adjust_camera_distance);
    }
}

fn synchronize_player_camera(
    settings: Res<CameraSettings>,
    players: Query<(&Transform, &ActorFigure), With<PlayerCharacter>>,
    mut cameras: Query<(&mut Transform, &CameraPosition), Without<PlayerCharacter>>,
) {
    let (player, figure) = players.single();
    let (mut camera, position) = cameras.single_mut();

    match position {
        CameraPosition::FirstPerson => {
            let rotation_matrix = Mat3::from_quat(player.rotation);
            camera.translation = player.translation + rotation_matrix * figure.eyes;
        }
        CameraPosition::ThirdPerson { distance } => {
            let rotation_matrix = Mat3::from_quat(camera.rotation);

            camera.translation = player.translation
                + rotation_matrix
                    * Vec3::new(
                        settings.offset.x,
                        settings.offset.y,
                        settings.offset.z + *distance,
                    );
        }
    }
}

/// Apply a periodic head bumping effect while the player is moving.
fn head_bumping(
    time: Res<Time>,
    settings: Res<CameraSettings>,
    players: Query<(&Transform, &Movement), With<PlayerCharacter>>,
    mut cameras: Query<(&mut Transform, &CameraPosition), Without<PlayerCharacter>>,
) {
    // Only apply head bumping when the player is moving.
    let Ok((player, movement)) = players.get_single() else {
        return;
    };

    let (mut camera, position) = cameras.single_mut();

    if position.is_third() {
        return;
    }

    // Relative distance between current and next frame.
    let distance = player.translation.distance(movement.desination).abs();

    // F
    let sc = time.elapsed_seconds() * PI * 2.0 * distance;
    let offset = sc.sin() * 0.1 * settings.head_bumping;

    camera.translation += Vec3::new(0.0, offset, 0.0);
}
