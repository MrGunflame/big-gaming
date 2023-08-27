//! Core UI systems

use bevy::prelude::{EventReader, Query, Res, ResMut, With};
use bevy::window::{PrimaryWindow, Window};

use game_common::components::actor::{ActorFlag, ActorFlags, Death};
use game_common::components::player::HostPlayer;
use game_common::scene::{Scene, SceneTransition, ServerError};
use game_input::CanMouseMove;

use crate::cursor::Cursor;
use crate::scenes::{Loading, MainMenu};
use crate::widgets::{Crosshair, Health, Weapon};
use crate::{widgets, InterfaceState};

pub fn capture_pointer_keys(
    mut mouse_move: ResMut<CanMouseMove>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut cursor: ResMut<Cursor>,
    state: Res<InterfaceState>,
    mut players: Query<&mut ActorFlags, With<HostPlayer>>,
) {
    let Ok(mut flags) = players.get_single_mut() else {
        return;
    };

    let mut window = windows.single_mut();

    if state.captures_pointer() {
        flags.remove(ActorFlag::CAN_ROTATE);
        flags.remove(ActorFlag::CAN_ATTACK);

        mouse_move.0 = false;

        cursor.unlock(&mut window);
    } else {
        flags.insert(ActorFlag::CAN_ROTATE);
        flags.insert(ActorFlag::CAN_ATTACK);

        mouse_move.0 = true;

        cursor.lock(&mut window);
    }

    if state.captures_keys() {
        flags.remove(ActorFlag::CAN_MOVE);
    } else {
        flags.insert(ActorFlag::CAN_MOVE);
    }

    // Reset the cursor to its pinned position. This does
    // nothing if the cursor is not currently pinned.
    cursor.reset(&mut window);
}

/// Create the death widget if the player dies.
pub fn death(mut state: ResMut<InterfaceState>, players: Query<&Death, With<HostPlayer>>) {
    if players.get_single().is_ok() {
        state.push(widgets::Death::default());
    }
}

pub fn scene_transition(
    mut state: ResMut<InterfaceState>,
    mut events: EventReader<SceneTransition>,
    mut cursor: ResMut<Cursor>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = windows.single_mut();

    for event in events.iter() {
        state.clear();

        // Only world locks cursor.
        if matches!(event.from, Scene::World) {
            cursor.unlock(&mut window);
        }

        match &event.to {
            Scene::Loading => {
                state.push(Loading::default());
            }
            Scene::MainMenu => {
                state.push(MainMenu::default());
            }
            Scene::World => {
                state.push(Health);
                state.push(Crosshair);
                state.push(Weapon);
            }
            Scene::ServerConnect { addr } => (),
            Scene::ServerError(err) => match err {
                ServerError::Connection(err) => {
                    state.push(crate::scenes::ServerError::new(err.clone()));
                }
            },
        }
    }
}