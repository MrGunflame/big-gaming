use bevy::prelude::*;
use bevy_rapier3d::prelude::{QueryFilter, RapierContext};
use common::components::combat::Damage;
use common::components::interaction::{InteractionQueue, Interactions};
use common::components::inventory::{Equipment, EquipmentSlot, Inventory};
use common::components::items::Item;
use common::components::player::FocusedEntity;

use crate::components::Rotation;
use crate::entities::actor::ActorFigure;
use crate::entities::player::PlayerCharacter;
use crate::entities::projectile::{Projectile, ProjectileBundle};
use crate::ui::{Focus, FocusKind};

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

// pub fn mouse_input(
//     mut events: EventReader<MouseMotion>,
//     mut camera: Query<&mut Rotation, With<Camera3d>>,
//     mut players: Query<&mut Rotation, (With<PlayerCharacter>, Without<Camera3d>)>,
// ) {
//     for event in events.iter() {
//         for (mut camera_rot, mut rotation) in camera.iter_mut().zip(players.iter_mut()) {
//             let yaw = event.delta.x * 0.1;
//             let pitch = event.delta.y * 0.1;

//             *camera_rot = camera_rot
//                 .add_yaw(Degrees(yaw))
//                 .saturating_add_pitch(Degrees(pitch));

//             *rotation = camera_rot.with_pitch(Radians(0.0));
//         }
//     }
// }

pub fn mouse_button_input(
    mut commands: Commands,
    rapier: Res<RapierContext>,
    assets: Res<AssetServer>,
    audio: Res<Audio>,
    mut players: Query<(&Transform, &mut Equipment, &ActorFigure, &Focus), With<PlayerCharacter>>,
    cameras: Query<&Rotation, With<Camera3d>>,
    projectiles: Query<(), With<Projectile>>,
    input: Res<Input<MouseButton>>,
    kb_input: Res<Input<KeyCode>>,
) {
    let (player, mut equipment, figure, focus) = players.single_mut();
    let camera_rot = cameras.single();

    if focus.kind != FocusKind::World {
        return;
    }

    let item = match equipment.get_mut(EquipmentSlot::MAIN_HAND) {
        Some(item) => item,
        None => return,
    };

    if kb_input.just_pressed(KeyCode::R) {
        if let Some(mag) = &mut item.magazine {
            if *mag == 0 {
                *mag = 30;
            } else {
                *mag = 31;
            }
        }
    }

    if input.pressed(MouseButton::Left) {
        if let Some(mag) = &mut item.magazine {
            if *mag == 0 {
                return;
            }

            *mag -= 1;
        } else {
            return;
        }

        audio.play_with_settings(
            assets.load("sounds/weapons/fire.wav"),
            PlaybackSettings::default().with_volume(0.03),
        );

        // Do a ray cast from the players camera position to figure out where to
        // shoot the projectile.
        let ray_origin = player.translation + figure.eyes;
        let ray_dir = camera_rot.movement_vec();
        let max_toi = 1000.0;
        let solid = true;
        let predicate = |entity| match projectiles.get(entity) {
            Ok(_) => false,
            Err(_) => true,
        };
        let filter = QueryFilter::new().predicate(&predicate);

        let target = match rapier.cast_ray(ray_origin, ray_dir, max_toi, solid, filter) {
            Some((_, toi)) => ray_origin + toi * ray_dir,
            None => ray_origin + max_toi * ray_dir,
        };

        let mut entity = ProjectileBundle::new(assets);

        // Create a new entity at the same position as the player,
        // pointing at the same direction as the player and a positive velocity
        // into the direction of the player.
        let mut origin = *player;
        origin.translation.y += 1.0;
        entity.scene.transform = origin.looking_at(target, Vec3::Y);
        entity.scene.transform.translation += camera_rot.movement_vec() * Vec3::splat(5.0);

        let dir = target - player.translation;
        entity.velocity.linvel = dir.normalize() * Vec3::splat(1000.0);

        commands.spawn(entity).insert(Damage::new(1));
    }
}

pub fn transform_system(mut entities: Query<(&mut Rotation, &mut Transform)>) {
    for (mut rotation, mut transform) in entities.iter_mut() {
        if rotation.modified {
            transform.rotation = rotation.to_quat();
            rotation.modified = false;
        }
    }
}

// TODO: This should scan for all entities `Interaction`s.
pub fn interact_target(
    mut queue: ResMut<InteractionQueue>,
    rapier: Res<RapierContext>,
    mut players: Query<(Entity, &mut FocusedEntity), With<PlayerCharacter>>,
    cameras: Query<(Entity, &Transform, &Rotation), With<Camera3d>>,
    entities: Query<(Entity, &Interactions)>,
    input: Res<Input<KeyCode>>,
) {
    let (player, mut focused_ent) = players.single_mut();
    let (cam, pos, rot) = cameras.single();

    let ray_pos = pos.translation;
    let ray_dir = rot.movement_vec();
    let max_toi = 4.0;
    let solid = true;
    let filter = QueryFilter::new().exclude_collider(cam);
    if let Some((entity, toi)) = rapier.cast_ray(ray_pos, ray_dir, max_toi, solid, filter) {
        if let Ok((entity, interactions)) = entities.get(entity) {
            *focused_ent = FocusedEntity::Some {
                entity,
                distance: toi,
            };

            if input.pressed(KeyCode::F) {
                queue.push(interactions.iter().nth(0).unwrap(), entity, player);
            }
        } else {
            *focused_ent = FocusedEntity::None;
        }
    } else {
        *focused_ent = FocusedEntity::None;
    }
}
