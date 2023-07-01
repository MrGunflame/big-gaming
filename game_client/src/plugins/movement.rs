use std::borrow::Cow;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::EventReader;
use bevy_ecs::query::{With, Without};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::{Query, Res, ResMut};
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::camera::CameraMode;
use game_common::components::player::HostPlayer;
use game_common::components::transform::Transform;
use game_core::time::Time;
use game_input::hotkeys::{
    Event, Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind,
};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_input::InputSet;
use game_net::snapshot::Command;
use game_window::cursor::Cursor;
use game_window::events::VirtualKeyCode;
use glam::{Quat, Vec3};

use crate::net::{NetSet, ServerConnection};
use crate::utils::extract_actor_rotation;

use super::camera::PrimaryCamera;

static mut MOVE_FORWARD: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("move_forward"),
    default: Key {
        trigger: TriggerKind::PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::W,
        },
    },
};

static mut MOVE_LEFT: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("move_left"),
    default: Key {
        trigger: TriggerKind::PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::A,
        },
    },
};

static mut MOVE_RIGHT: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("move_right"),
    default: Key {
        trigger: TriggerKind::PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::D,
        },
    },
};

static mut MOVE_BACK: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("move_back"),
    default: Key {
        trigger: TriggerKind::PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::S,
        },
    },
};

static mut SPRINT: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("sprint"),
    default: Key {
        trigger: TriggerKind::JUST_PRESSED.and(TriggerKind::JUST_RELEASED),
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::LShift,
        },
    },
};

static mut JUMP: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("jump"),
    default: Key {
        trigger: TriggerKind::JUST_PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::Space,
        },
    },
};

pub struct MovementEvent;

impl MovementEvent {
    fn forward(event: &Event) -> bool {
        event.id == unsafe { MOVE_FORWARD.id }
    }

    fn back(event: &Event) -> bool {
        event.id == unsafe { MOVE_BACK.id }
    }

    fn left(event: &Event) -> bool {
        event.id == unsafe { MOVE_LEFT.id }
    }

    fn right(event: &Event) -> bool {
        event.id == unsafe { MOVE_RIGHT.id }
    }
}

impl HotkeyFilter for MovementEvent {
    fn filter(id: HotkeyId) -> bool {
        unsafe {
            for other in [MOVE_FORWARD.id, MOVE_BACK.id, MOVE_LEFT.id, MOVE_RIGHT.id] {
                if id == other {
                    return true;
                }
            }
        }

        false
    }
}

struct Sprint;

impl HotkeyFilter for Sprint {
    fn filter(id: HotkeyId) -> bool {
        id == unsafe { SPRINT.id }
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

struct JumpEvent;

impl HotkeyFilter for JumpEvent {
    fn filter(id: HotkeyId) -> bool {
        id == unsafe { JUMP.id }
    }
}

fn register_events(mut hotkeys: ResMut<Hotkeys>) {
    let mut move_forward = unsafe { &mut MOVE_FORWARD };
    let id = hotkeys.register(move_forward.clone());
    move_forward.id = id;

    drop(move_forward);

    let mut move_back = unsafe { &mut MOVE_BACK };
    let id = hotkeys.register(move_back.clone());
    move_back.id = id;
    drop(move_back);

    let mut move_left = unsafe { &mut MOVE_LEFT };
    let id = hotkeys.register(move_left.clone());
    move_left.id = id;
    drop(move_left);

    let mut move_right = unsafe { &mut MOVE_RIGHT };
    let id = hotkeys.register(move_right.clone());
    move_right.id = id;
    drop(move_right);

    let mut sprint = unsafe { &mut SPRINT };
    let id = hotkeys.register(sprint.clone());
    sprint.id = id;
    drop(sprint);

    let mut jump = unsafe { &mut JUMP };
    let id = hotkeys.register(jump.clone());
    jump.id = id;
    drop(jump);
}

pub fn translation_events(
    mut conn: ResMut<ServerConnection>,
    time: Res<Time>,
    mut players: Query<(&mut Transform, &MovementSpeed), With<HostPlayer>>,
    mut cameras: Query<(&mut Transform, &CameraMode), (Without<HostPlayer>, With<PrimaryCamera>)>,
    mut events: HotkeyReader<MovementEvent>,
) {
    let (mut camera, mode) = cameras.single_mut();

    let Ok((mut transform, speed)) = players.get_single_mut() else {
        return;
    };

    let mut angle = Angle::default();

    for event in events.iter() {
        if MovementEvent::forward(event) {
            angle.front();
        }

        if MovementEvent::back(event) {
            angle.back();
        }

        if MovementEvent::left(event) {
            angle.left();
        }

        if MovementEvent::right(event) {
            angle.right();
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
                let direction =
                    transform.rotation * Quat::from_axis_angle(Vec3::Y, angle) * -Vec3::Z;

                let distance = direction * speed.0 * delta;
                transform.translation += distance;

                let id = conn.host;
                conn.send(Command::EntityTranslate {
                    id,
                    translation: transform.translation,
                });
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
        let id = conn.host;

        conn.send(Command::EntityRotate {
            id,
            rotation: props.rotation,
        });

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
