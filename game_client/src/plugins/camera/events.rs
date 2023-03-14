use std::borrow::Cow;
use std::mem::MaybeUninit;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::{EventReader, KeyCode, Query, Res, ResMut};
use bevy::time::Time;
use game_common::components::camera::CameraMode;
use game_input::hotkeys::{
    Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, TriggerKind,
};

use crate::entities::player::CameraPosition;
use crate::plugins::hotkeys::{Event, EventId, HotkeyStore};

const DEFAULT_TRIGGER_TOGGLE_CAMERA: KeyCode = KeyCode::V;

static mut TOGGLE_CAMERA: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("toggle_camera"),
    default: game_input::hotkeys::Key {
        trigger: TriggerKind::JUST_PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::V,
        },
    },
};

pub(super) fn register_events(mut hotkeys: ResMut<Hotkeys>) {
    let mut toggle = unsafe { &mut TOGGLE_CAMERA };
    let id = hotkeys.register(toggle.clone());
    toggle.id = id;
    drop(toggle);
}

pub struct ToggleCamera;

impl HotkeyFilter for ToggleCamera {
    fn filter(id: HotkeyId) -> bool {
        id == unsafe { TOGGLE_CAMERA.id }
    }
}

pub(super) fn toggle_camera_position(
    mut cameras: Query<&mut CameraMode>,
    mut events: HotkeyReader<ToggleCamera>,
) {
    let mut mode = cameras.single_mut();

    for _ in events.iter() {
        *mode = match *mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson { distance: 5.0 },
            CameraMode::ThirdPerson { distance: _ } => CameraMode::FirstPerson,
        };
    }
}

// pub(super) fn adjust_camera_distance(
//     time: Res<Time>,
//     mut cameras: Query<&mut CameraPosition>,
//     mut events: EventReader<MouseWheel>,
// ) {
//     let mut position = cameras.single_mut();

//     let delta = time.delta_seconds();

//     if let CameraPosition::ThirdPerson { distance } = &mut *position {
//         for event in events.iter() {
//             *distance -= event.y * delta * 5.0;

//             if *distance < 1.0 {
//                 *distance = 1.0;
//             } else if *distance > 10.0 {
//                 *distance = 10.0;
//             }
//         }
//     }
// }
