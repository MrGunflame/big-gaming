use std::borrow::Cow;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::EventReader;
use bevy_ecs::query::{With, Without};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::camera::CameraMode;
use game_common::components::player::HostPlayer;
use game_common::components::transform::Transform;
use game_core::time::Time;
use game_input::hotkeys::{Hotkey, HotkeyCode, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_input::InputSet;
use game_net::proto::MoveBits;
use game_net::snapshot::{Command, EntityRotate, EntityTranslate, PlayerMove};
use game_window::cursor::Cursor;
use game_window::events::VirtualKeyCode;
use glam::{Quat, Vec3};

use crate::net::{NetSet, ServerConnection};
use crate::utils::extract_actor_rotation;

use super::camera::PrimaryCamera;

#[derive(Resource)]
pub struct MovementHotkeys {
    forward: Hotkey,
    back: Hotkey,
    left: Hotkey,
    right: Hotkey,
}

impl Default for MovementHotkeys {
    fn default() -> Self {
        Self {
            forward: Hotkey {
                id: HotkeyId(0),
                name: Cow::Borrowed("move_forward"),
                default: Key {
                    trigger: TriggerKind::PRESSED,
                    code: HotkeyCode::KeyCode {
                        key_code: KeyCode::W,
                    },
                },
            },
            back: Hotkey {
                id: HotkeyId(0),
                name: Cow::Borrowed("move_back"),
                default: Key {
                    trigger: TriggerKind::PRESSED,
                    code: HotkeyCode::KeyCode {
                        key_code: KeyCode::S,
                    },
                },
            },
            left: Hotkey {
                id: HotkeyId(0),
                name: Cow::Borrowed("move_left"),
                default: Key {
                    trigger: TriggerKind::PRESSED,
                    code: HotkeyCode::KeyCode {
                        key_code: KeyCode::A,
                    },
                },
            },
            right: Hotkey {
                id: HotkeyId(0),
                name: Cow::Borrowed("move_right"),
                default: Key {
                    trigger: TriggerKind::PRESSED,
                    code: HotkeyCode::KeyCode {
                        key_code: KeyCode::D,
                    },
                },
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum MovementSet {
    Read,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovementHotkeys::default());
        app.add_startup_system(register_events);

        app.add_system(translation_events.in_set(MovementSet::Read));
        app.add_system(rotation_events.in_set(MovementSet::Read));

        // Run transform updates AFTER inputs are ready, but BEFORE
        // updating server events.
        app.configure_set(MovementSet::Read.after(InputSet::Hotkeys));
        app.configure_set(MovementSet::Read.before(NetSet::first()));

        app.add_system(lock_mouse);
    }
}

fn register_events(mut hotkeys: ResMut<Hotkeys>, mut movement_hotkeys: ResMut<MovementHotkeys>) {
    movement_hotkeys.forward.id = hotkeys.register(movement_hotkeys.forward.clone());
    movement_hotkeys.back.id = hotkeys.register(movement_hotkeys.back.clone());
    movement_hotkeys.left.id = hotkeys.register(movement_hotkeys.left.clone());
    movement_hotkeys.right.id = hotkeys.register(movement_hotkeys.right.clone());
}

pub fn translation_events(
    mut conn: ResMut<ServerConnection>,
    time: Res<Time>,
    mut players: Query<(&mut Transform, &MovementSpeed), With<HostPlayer>>,
    mut cameras: Query<(&mut Transform, &CameraMode), (Without<HostPlayer>, With<PrimaryCamera>)>,
    mut events: HotkeyReader<Hotkey>,
    movement_hotkeys: Res<MovementHotkeys>,
) {
    let (mut camera, mode) = cameras.single_mut();

    let Ok((mut transform, speed)) = players.get_single_mut() else {
        return;
    };

    let mut angle = Angle::default();

    let mut forward = false;
    let mut back = false;
    let mut left = false;
    let mut right = false;

    for event in events.iter() {
        if event.id == movement_hotkeys.forward.id {
            angle.front();
            forward = true;
        }

        if event.id == movement_hotkeys.back.id {
            angle.back();
            back = true;
        }

        if event.id == movement_hotkeys.left.id {
            angle.left();
            left = true;
        }

        if event.id == movement_hotkeys.right.id {
            angle.right();
            right = true;
        }
    }

    if let Some(angle) = angle.to_radians() {
        let delta = time.delta().as_secs_f32();

        match mode {
            // In detached mode control the camera directly.
            CameraMode::Detached => {
                let direction = camera.rotation * Quat::from_axis_angle(Vec3::Y, angle) * -Vec3::Z;

                let distance = direction * 1.0 * delta;
                camera.translation += distance;
            }
            // Otherwise control the player actor.
            _ => {
                let entity_id = conn.server_entities.get(conn.host).unwrap();
                let bits = MoveBits {
                    forward,
                    back,
                    left,
                    right,
                };

                conn.send(Command::PlayerMove(PlayerMove {
                    entity: entity_id,
                    bits,
                }));

                let speed = 1.0;
                let dir = (bits.forward as u8 as f32) * -Vec3::Z
                    + (bits.back as u8 as f32) * Vec3::Z
                    + (bits.left as u8 as f32) * -Vec3::X
                    + (bits.right as u8 as f32) * Vec3::X;
                let delta = transform.rotation * dir * speed;
                transform.translation += delta;

                dbg!(transform.translation);
            }
        }
    }
}

pub fn rotation_events(
    mut conn: ResMut<ServerConnection>,
    mut events: EventReader<MouseMotion>,
    mut players: Query<(&mut ActorProperties, &mut Transform), With<HostPlayer>>,
    mut cameras: Query<(&mut Transform, &CameraMode), (Without<HostPlayer>, With<PrimaryCamera>)>,
) {
    let (mut camera, mode) = cameras.single_mut();

    let Ok((mut props, mut player)) = players.get_single_mut() else {
        return;
    };

    // true if we need to notify the server about the new value.
    let mut is_changed = false;

    for event in events.iter() {
        let yaw = event.delta.x * 0.001;
        let pitch = event.delta.y * 0.001;

        let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
        let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

        match mode {
            CameraMode::Detached => {
                camera.rotation = q1 * camera.rotation;
                camera.rotation = camera.rotation * q2;

                if !camera.rotation.is_normalized() {
                    camera.rotation = camera.rotation.normalize();
                }
            }
            _ => {
                props.rotation = q1 * props.rotation;
                props.rotation = props.rotation * q2;

                if !props.rotation.is_normalized() {
                    props.rotation = props.rotation.normalize();
                }

                is_changed = true;
            }
        }
    }

    if is_changed {
        let entity_id = conn.server_entities.get(conn.host).unwrap();

        conn.send(Command::EntityRotate(EntityRotate {
            id: entity_id,
            rotation: props.rotation,
        }));

        player.rotation = extract_actor_rotation(props.rotation);
    }
}

/// The movement angle based on four input directions.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
struct Angle(u8);

impl Angle {
    const FRONT: Self = Self(1);
    const BACK: Self = Self(1 << 1);
    const LEFT: Self = Self(1 << 2);
    const RIGHT: Self = Self(1 << 3);

    fn front(&mut self) {
        self.0 |= Self::FRONT.0;
    }

    fn back(&mut self) {
        self.0 |= Self::BACK.0;
    }

    fn left(&mut self) {
        self.0 |= Self::LEFT.0;
    }

    fn right(&mut self) {
        self.0 |= Self::RIGHT.0;
    }

    fn to_degrees(self) -> Option<f32> {
        let (front, back, left, right) = (
            self.0 & Self::FRONT.0 != 0,
            self.0 & Self::BACK.0 != 0,
            self.0 & Self::LEFT.0 != 0,
            self.0 & Self::RIGHT.0 != 0,
        );

        match (front, back, left, right) {
            // Single
            (true, false, false, false) => Some(0.0),
            (false, true, false, false) => Some(180.0),
            (false, false, true, false) => Some(90.0),
            (false, false, false, true) => Some(270.0),
            // Front
            (true, false, true, false) => Some(45.0),
            (true, false, false, true) => Some(315.0),
            (true, false, true, true) => Some(0.0),
            // Back
            (false, true, true, false) => Some(135.0),
            (false, true, false, true) => Some(225.0),
            (false, true, true, true) => Some(180.0),
            // Locked
            (true, true, false, false) => None,
            (true, true, true, false) => Some(90.0),
            (true, true, false, true) => Some(270.0),
            (true, true, true, true) => None,

            (false, false, true, true) => None,
            (false, false, false, false) => None,
        }
    }

    fn to_radians(self) -> Option<f32> {
        self.to_degrees().map(f32::to_radians)
    }
}

fn lock_mouse(
    mut cursor: ResMut<Cursor>,
    mut events: EventReader<KeyboardInput>,
    mut cameras: Query<&mut CameraMode>,
) {
    for event in events.iter().filter(|e| e.state.is_pressed()) {
        if let Some(VirtualKeyCode::Escape) = event.key_code {
            if cursor.is_locked() {
                cursor.unlock()
            } else {
                cursor.lock()
            }
        }

        if let Some(VirtualKeyCode::Tab) = event.key_code {
            for mut mode in &mut cameras {
                if mode.is_detached() {
                    *mode = CameraMode::FirstPerson;
                } else {
                    *mode = CameraMode::Detached;
                }
            }
        }
    }
}
