mod events;

use bevy::prelude::{EulerRot, Mat3, Plugin, Query, Transform, Vec3, With, Without};

use crate::components::Rotation;
use crate::entities::actor::ActorFigure;
use crate::entities::player::{CameraPosition, PlayerCharacter};

use self::events::{adjust_camera_distance, register_events, toggle_camera_position};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events)
            .add_system(crate::systems::input::transform_system)
            .add_system(crate::systems::input::mouse_button_input)
            .add_system(crate::systems::input::interact_target)
            .add_system(synchronize_player_camera)
            .add_system(toggle_camera_position)
            .add_system(adjust_camera_distance);
    }
}

fn synchronize_player_camera(
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

            camera.translation =
                player.translation + rotation_matrix * Vec3::new(0.0, 0.0, *distance);
        }
    }
}
