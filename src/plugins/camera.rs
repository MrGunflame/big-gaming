mod events;

use bevy::prelude::{Mat3, Plugin, Query, Transform, Vec3, With, Without};

use crate::entities::player::{CameraPosition, PlayerCharacter};

use self::events::{register_events, toggle_camera_position};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events)
            .add_system(crate::systems::input::grab_mouse)
            .add_system(crate::systems::input::mouse_input)
            .add_system(crate::systems::input::mouse_scroll)
            .add_system(crate::systems::input::transform_system)
            .add_system(crate::systems::input::mouse_button_input)
            .add_system(synchronize_player_camera)
            .add_system(toggle_camera_position);
    }
}

fn synchronize_player_camera(
    players: Query<&Transform, With<PlayerCharacter>>,
    mut cameras: Query<(&mut Transform, &CameraPosition), Without<PlayerCharacter>>,
) {
    let player = players.single();
    let (mut camera, position) = cameras.single_mut();

    match position {
        CameraPosition::FirstPerson => {
            // Camera is slightly higher than player feet.
            let offset = Vec3 {
                x: 0.0,
                y: 1.8,
                z: 0.0,
            };

            camera.translation = player.translation + offset;
        }
        CameraPosition::ThirdPerson { distance } => {
            let rotation_matrix = Mat3::from_quat(camera.rotation);

            camera.translation =
                player.translation + rotation_matrix * Vec3::new(0.0, 0.0, *distance);
        }
    }
}
