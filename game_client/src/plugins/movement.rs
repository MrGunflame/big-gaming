mod events;

use std::borrow::Cow;

use bevy::prelude::{
    Camera3d, Commands, Entity, EventReader, IntoSystemConfig, IntoSystemSetConfig, KeyCode,
    Plugin, Quat, Query, ResMut, SystemSet, Vec3, With, Without,
};
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::movement::{Jump, Movement, Rotate, RotateQueue};
use game_common::components::player::HostPlayer;
use game_input::hotkeys::{
    Event, Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind,
};
use game_input::mouse::MouseMotion;
use game_input::InputSet;

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
        trigger: TriggerKind::JUST_PRESSED | TriggerKind::JUST_RELEASED,
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
    Apply,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events);

        app.add_system(movement_events.in_set(MovementSet::Read));
        app.add_system(mouse_movement.in_set(MovementSet::Read));
        app.add_system(toggle_sprint.in_set(MovementSet::Read));
        app.add_system(jump_events.in_set(MovementSet::Read));

        app.add_system(events::handle_movement_events.in_set(MovementSet::Apply));
        app.add_system(events::handle_rotate_events.in_set(MovementSet::Apply));
        // app.add_system(events::handle_jump_events.in_set(MovementSet::Apply));

        app.configure_set(InputSet::Hotkeys.before(MovementSet::Read));
        app.configure_set(MovementSet::Read.before(MovementSet::Apply));
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

fn toggle_sprint(
    mut players: Query<&mut MovementSpeed, With<HostPlayer>>,
    mut events: HotkeyReader<Sprint>,
) {
    let Ok(mut speed) = players.get_single_mut() else {
        return;
    };

    for event in events.iter() {
        if event.trigger.just_pressed() {
            **speed = 5.0;
        } else {
            **speed = 3.0;
        }
    }
}

fn movement_events(
    mut commands: Commands,
    mut events: HotkeyReader<MovementEvent>,
    players: Query<Entity, With<HostPlayer>>,
) {
    let Ok(entity) = players.get_single() else {
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

        if MovementEvent::right(event) {
            angle.right();
        }

        if MovementEvent::left(event) {
            angle.left();
        }
    }

    if let Some(angle) = angle.to_radians() {
        commands.entity(entity).insert(Movement {
            direction: Quat::from_axis_angle(Vec3::Y, angle),
        });
    }
}

fn jump_events(
    mut commands: Commands,
    players: Query<Entity, With<HostPlayer>>,
    mut events: HotkeyReader<JumpEvent>,
) {
    let Ok(entity) = players.get_single() else {
        return;
    };

    for _ in events.iter() {
        commands.entity(entity).insert(Jump);
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

fn mouse_movement(
    mut events: EventReader<MouseMotion>,
    mut players: Query<
        (&mut RotateQueue, &mut ActorProperties),
        (With<HostPlayer>, Without<Camera3d>),
    >,
) {
    let Ok((mut queue, mut props)) = players.get_single_mut() else {
        return;
    };

    let mut changed = false;

    for event in events.iter() {
        let yaw = event.delta.x * 0.001;
        let pitch = event.delta.y * 0.001;

        // let yaw = camera.rotation.yaw() - yaw;
        // let mut pitch = camera.rotation.pitch() - pitch;

        // if pitch < -(PI / 2.0) {
        //     pitch = -(PI / 2.0);
        // } else if pitch > PI / 2.0 {
        //     pitch = PI / 2.0;
        // }

        // let quat = camera.rotation.with_yaw(yaw).with_pitch(pitch);
        // let quat = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

        let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
        let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

        // let quat = Quat::from_euler(EulerRot::YXZ, y - yaw, pitch, z);

        // camera.rotation.to_axis_angle();
        // camera.rotate_axis(-Vec3::Y, yaw);
        // camera.rotate(quat);

        // camera.rotation = q1 * camera.rotation;
        // camera.rotation = camera.rotation * q2;
        props.rotation = q1 * props.rotation;
        props.rotation = props.rotation * q2;

        // *camera_rot = camera_rot
        //     .add_yaw(Degrees(yaw))
        //     .saturating_add_pitch(Degrees(pitch));

        // *player_rot = camera_rot.with_pitch(Radians(0.0));
        // player.rotation = q1 * player.rotation;
        // commands.entity(entity).insert(Rotate {
        //     destination: q1 * player.rotation,
        // });

        // player.rotation = q1 * player.rotation;
        // rotation = q1 * rotation;
        // rotation = rotation * q2;

        changed = true;
    }

    if changed {
        queue.0.push_back(Rotate {
            destination: props.rotation,
        });
    }
}
