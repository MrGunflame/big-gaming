use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_rapier3d::prelude::*;

use crate::components::Rotation;
use crate::entities::player::{CameraPosition, PlayerCharacter};
use crate::entities::projectile::ProjectileBundle;
use crate::plugins::combat::Damage;
use crate::utils::{Degrees, Radians};

pub fn grab_mouse(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();

    // window.set_cursor_visibility(false);
    // window.set_cursor_grab_mode(CursorGrabMode::Locked);
}

// pub fn keyboard_input(
//     rapier_ctx: Res<RapierContext>,
//     hotkeys: Res<HotkeyStore>,
//     input: Res<Input<KeyCode>>,
//     mut camera: Query<(&mut Transform, &mut CameraPosition), With<Camera3d>>,
//     mut players: Query<
//         (Entity, &mut Transform, &Rotation, &mut Velocity, &Collider),
//         (With<PlayerCharacter>, Without<Camera3d>),
//     >,
// ) {
//     for (
//         (mut camera, mut camera_position),
//         (entity, mut player, rotation, mut velocity, collider),
//     ) in camera.iter_mut().zip(players.iter_mut())
//     {
//         let shape_pos = player.translation;
//         let shape_rot = player.rotation;
//         let is_on_ground = || {
//             let shape_vel = -Vec3::Y;
//             let max_toi = 2.0;
//             let filter = QueryFilter::new().exclude_collider(entity);

//             rapier_ctx
//                 .cast_shape(shape_pos, shape_rot, shape_vel, &collider, max_toi, filter)
//                 .is_some()
//         };

//         if hotkeys.pressed::<MoveLeft>(&input) {
//             let vec = rotation.left(Degrees(90.0)).movement_vec() * 0.2;
//             player.translation += vec;
//         }

//         if hotkeys.pressed::<MoveRight>(&input) {
//             let vec = rotation.right(Degrees(90.0)).movement_vec() * 0.2;
//             player.translation += vec;
//         }

//         if hotkeys.pressed::<MoveBackward>(&input) {
//             let vec = rotation.left(Degrees(180.0)).movement_vec() * 0.2;
//             player.translation += vec;
//         }

//         if hotkeys.pressed::<MoveForward>(&input) {
//             let vec = rotation.movement_vec() * 0.2;
//             player.translation += vec;
//         }

//         if input.just_pressed(KeyCode::V) {
//             println!("swapped");

//             *camera_position = match *camera_position {
//                 CameraPosition::FirstPerson => {
//                     camera.translation.y += 5.0;
//                     camera.rotation = Quat::from_axis_angle(Vec3::Y, 0.0);

//                     CameraPosition::ThirdPerson { distance: 5.0 }
//                 }
//                 CameraPosition::ThirdPerson { distance: _ } => {
//                     camera.translation = player.translation;

//                     camera.translation.z += 0.5;
//                     camera.translation.y += 1.8;

//                     CameraPosition::FirstPerson
//                 }
//             };
//         }

//         if input.just_pressed(KeyCode::Space) {
//             if is_on_ground() {
//                 velocity.linvel.y += 10.0;
//             }
//         }
//     }
// }

pub fn mouse_input(
    mut events: EventReader<MouseMotion>,
    mut camera: Query<(&mut Rotation, &CameraPosition), With<Camera3d>>,
    mut players: Query<(&mut Transform, &mut Rotation), (With<PlayerCharacter>, Without<Camera3d>)>,
) {
    for event in events.iter() {
        for ((mut camera_rot, camera_pos), (mut player, mut rotation)) in
            camera.iter_mut().zip(players.iter_mut())
        {
            let yaw = event.delta.x;
            let pitch = event.delta.y;

            *camera_rot = camera_rot
                .add_yaw(Degrees(yaw))
                .saturating_add_pitch(Degrees(pitch));

            *rotation = camera_rot.with_pitch(Radians(0.0));
        }
    }
}

pub fn mouse_button_input(
    mut commands: Commands,
    assets: Res<AssetServer>,
    players: Query<(&Transform, &Rotation), With<PlayerCharacter>>,
    cameras: Query<&Rotation, With<Camera3d>>,
    input: Res<Input<MouseButton>>,
) {
    let (player, rot) = players.single();
    let camera_rot = cameras.single();

    if input.pressed(MouseButton::Left) {
        let mut entity = ProjectileBundle::new(assets);

        // Create a new entity at the same position as the player,
        // pointing at the same direction as the player and a positive velocity
        // into the direction of the player.
        entity.scene.transform.translation = player.translation;
        entity.scene.transform.rotation = player.rotation;
        entity.scene.transform.translation += camera_rot.movement_vec() * Vec3::splat(5.0);
        entity.velocity.linvel = camera_rot.movement_vec() * Vec3::splat(1000.0);

        commands.spawn(entity).insert(Damage::new(1));
    }
}

pub fn mouse_scroll(
    mut events: EventReader<MouseWheel>,
    mut players: Query<(&mut CameraPosition), With<Camera3d>>,
) {
    if let Ok(mut position) = players.get_single_mut() {
        if let CameraPosition::ThirdPerson { distance } = &mut *position {
            for event in events.iter() {
                *distance -= event.y;
            }
        }
    }
}

pub fn transform_system(mut entities: Query<(&Rotation, &mut Transform)>) {
    for (rotation, mut transform) in entities.iter_mut() {
        transform.rotation = rotation.to_quat();
    }
}

pub fn sync_player_camera(
    mut players: Query<&Transform, With<PlayerCharacter>>,
    mut cameras: Query<(&mut Transform, &CameraPosition), Without<PlayerCharacter>>,
) {
    for player in &mut players {
        for (mut camera, pos) in &mut cameras {
            update_player_camera(&mut camera, player, *pos);
        }
    }
}

/// Update the player camera using the players position.
fn update_player_camera(camera: &mut Transform, player: &Transform, pos: CameraPosition) {
    match pos {
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
                player.translation + rotation_matrix * Vec3::new(0.0, 0.0, distance);
        }
    }
}
