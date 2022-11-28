use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::components::Rotation;
use crate::entities::player::{CameraPosition, PlayerCharacter};
use crate::utils::{Degrees, Radians};

pub fn keyboard_input(
    input: Res<Input<KeyCode>>,
    mut camera: Query<(&mut Transform, &mut CameraPosition), With<Camera3d>>,
    mut players: Query<
        (&mut Transform, &Rotation, &Velocity),
        (With<PlayerCharacter>, Without<Camera3d>),
    >,
) {
    // let mut movement_vec = Vec3::ZERO;

    // if input.pressed(KeyCode::A) {
    //     let vec = rotation.left(90.0).movement_vec() * velocity.as_f32();

    //     // camera.translation.z -= vec.y;
    //     // camera.translation.x -= vec.x;

    //     movement_vec += vec;
    // }

    // if input.pressed(KeyCode::D) {
    //     let vec = rotation.right(90.0).movement_vec() * velocity.as_f32();

    //     // camera.translation.z -= vec.y;
    //     // camera.translation.x -= vec.x;

    //     movement_vec -= vec;
    // }

    // if input.pressed(KeyCode::S) {
    //     let vec = rotation.movement_vec() * velocity.as_f32();

    //     // camera.translation.z += vec.y;
    //     // camera.translation.x += vec.x;

    //     movement_vec += vec;
    // }

    // if input.pressed(KeyCode::W) {
    //     let vec = rotation.movement_vec() * velocity.as_f32();

    //     // camera.translation.z -= vec.y;
    //     // camera.translation.x -= vec.x;

    //     movement_vec -= vec;
    // }

    // for mut player in set.p0().iter_mut() {
    //     player.translation = movement_vec;
    // }

    // for (mut camera, rotation, velocity, camera_position) in set.p1().iter_mut() {
    //     match camera_position {
    //         CameraPosition::FirstPerson => {
    //             camera.translation = movement_vec;
    //         }
    //         CameraPosition::ThirdPerson => {
    //             movement_vec.x += 5.0;
    //             movement_vec.y += 5.0;
    //             movement_vec.z += 5.0;

    //             camera.translation = movement_vec;
    //         }
    //     }
    // }

    for ((mut camera, mut camera_position), (mut player, rotation, velocity)) in
        camera.iter_mut().zip(players.iter_mut())
    {
        // println!("PLAYER {:?}", player);
        // println!("CAMERA {:?}", camera);

        if input.pressed(KeyCode::A) {
            let vec = rotation.left(Degrees(90.0)).movement_vec() * 0.2;

            // camera.translation.z -= vec.y;
            // camera.translation.x -= vec.x;
            update_player_position(&mut player, &mut camera, *camera_position, vec);
        }

        if input.pressed(KeyCode::D) {
            let vec = rotation.right(Degrees(90.0)).movement_vec() * 0.2;

            // camera.translation.z -= vec.y;
            // camera.translation.x -= vec.x;
            update_player_position(&mut player, &mut camera, *camera_position, vec);
        }

        if input.pressed(KeyCode::S) {
            let vec = rotation.left(Degrees(180.0)).movement_vec() * 0.2;

            // camera.translation.z += vec.y;
            // camera.translation.x += vec.x;
            update_player_position(&mut player, &mut camera, *camera_position, vec);
        }

        if input.pressed(KeyCode::W) {
            let vec = rotation.movement_vec() * 0.2;
            update_player_position(&mut player, &mut camera, *camera_position, vec);
        }

        if input.just_pressed(KeyCode::V) {
            println!("swapped");

            *camera_position = match *camera_position {
                CameraPosition::FirstPerson => {
                    camera.translation.y += 5.0;
                    camera.rotation = Quat::from_axis_angle(Vec3::Y, 0.0);

                    CameraPosition::ThirdPerson
                }
                CameraPosition::ThirdPerson => {
                    camera.translation = player.translation;

                    camera.translation.z += 0.5;
                    camera.translation.y += 1.8;

                    CameraPosition::FirstPerson
                }
            };
        }
    }
}

pub fn mouse_input(
    mut events: EventReader<MouseMotion>,
    mut camera: Query<(&mut Transform, &mut Rotation, &CameraPosition), With<Camera3d>>,
    mut players: Query<(&mut Transform, &mut Rotation), (With<PlayerCharacter>, Without<Camera3d>)>,
) {
    for event in events.iter() {
        for ((mut camera, mut camera_rot, camera_pos), (mut player, mut rotation)) in
            camera.iter_mut().zip(players.iter_mut())
        {
            let yaw = event.delta.x;
            let pitch = event.delta.y;

            // camera_rot.yaw -= yaw * 0.2;
            // camera_rot.pitch += pitch * 0.2;

            // rotation.yaw -= yaw * 0.2;

            // player.rotation = rotation.to_quat();

            match camera_pos {
                CameraPosition::ThirdPerson => {
                    // camera.rotation = player.rotation;
                    // camera.translation = player.translation;
                    // camera.translation.y += 5.0;
                    //camera.rotate_around(player.translation, camera_rot.to_quat());

                    // let yaw = Quat::from_rotation_y(-event.delta.x.to_radians() * 0.1);
                    // let yaw = Quat::from_rotation_y(-yaw * 0.2);
                    let pitch = Quat::from_rotation_x(event.delta.y.to_radians() * 0.1);

                    // camera.rotation = yaw * camera.rotation;
                    // camera.rotation =
                    //     camera.rotation * Quat::from_axis_angle(-Vec3::Y, yaw.to_radians() * 0.2);

                    // camera.rotation = pitch * camera.rotation;

                    // let rotation_matrix = Mat3::from_quat(camera.rotation);
                    // camera.translation =
                    //     player.translation + rotation_matrix.mul_vec3(Vec3::new(0.0, 0.0, 5.0));
                }
                // Player rotation is Camera rotation with y offset.
                CameraPosition::FirstPerson => {
                    *camera_rot = camera_rot
                        .add_yaw(Degrees(yaw))
                        .saturating_add_pitch(Degrees(pitch));

                    camera.rotation = camera_rot.to_quat();

                    // The entity doesn't change pitch.
                    *rotation = camera_rot.with_pitch(Radians(0.0));
                    player.rotation = rotation.to_quat();

                    // camera_rot = camera_rot.camera_rot.yaw -= yaw * 0.2;
                    // camera_rot.pitch += pitch * 0.2;

                    // rotation.yaw -= yaw * 0.2;
                    // rotation.pitch += pitch * 0.2;

                    // println!("{:?}", camera.rotation);

                    // // camera.rotation =
                    // //     camera.rotation * Quat::from_axis_angle(-Vec3::X, pitch.to_radians() * 0.2);
                    // println!("{:?}", camera.rotation);

                    // println!("Rotation {:?}", camera_rot);

                    // let mat = Mat3::from_axis_angle(-Vec3::X, camera_rot.pitch.to_radians())
                    //     * Mat3::from_axis_angle(-Vec3::Y, camera_rot.yaw.to_radians());

                    // camera.rotation = Quat::from_mat3(&mat);

                    // let mat = Mat3::from_euler(
                    //     EulerRot::YXZ,
                    //     camera_rot.yaw.to_radians(),
                    //     -camera_rot.pitch.to_radians(),
                    //     0.0,
                    // );

                    // camera.rotation = Quat::from_mat3(&mat);

                    // camera.rotation = Quat::from_euler(
                    //     EulerRot::XYZ,
                    //     camera_rot.pitch.to_radians(),
                    //     camera_rot.yaw.to_radians(),
                    //     0.0,
                    // );

                    // camera.rotation =
                    //     Quat::from_axis_angle(-Vec3::X, camera_rot.pitch.to_radians())
                    //         * Quat::from_axis_angle(-Vec3::Y, camera_rot.yaw.to_radians());

                    // camera.rotation =
                    //     camera.rotation * Quat::from_axis_angle(-Vec3::Y, yaw.to_radians() * 0.2);

                    // camera.rotation =
                    //     camera.rotation * Quat::from_axis_angle(-Vec3::Y, yaw.to_radians());

                    // camera.rotation =
                    //     camera.rotation * Quat::from_axis_angle(-Vec3::X, pitch.to_radians() * 0.2);
                    // camera.rotation =
                    //     camera.rotation * Quat::from_axis_angle(-Vec3::Y, yaw.to_radians() * 0.2);

                    // player.rotation =
                    //     player.rotation * Quat::from_axis_angle(-Vec3::Y, yaw.to_radians() * 0.2);

                    //player.rotation = rotation.to_quat();
                    //camera.rotation = camera_rot.to_quat();
                }
            }
        }
    }
}

fn update_player_position(
    player: &mut Transform,
    camera: &mut Transform,
    camera_position: CameraPosition,
    vec: Vec3,
) {
    player.translation += vec;

    match camera_position {
        CameraPosition::FirstPerson => {
            camera.translation += vec;
        }
        CameraPosition::ThirdPerson => {}
    }
}
