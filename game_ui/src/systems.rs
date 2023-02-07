//! Core UI systems

use bevy::prelude::{Query, Res, ResMut, With};
use bevy::window::Windows;

use game_common::components::actor::{ActorFlag, ActorFlags, Death};
use game_common::components::player::HostPlayer;
use game_input::CanMouseMove;

use crate::cursor::Cursor;
use crate::{widgets, InterfaceState};

pub fn capture_pointer_keys(
    mut mouse_move: ResMut<CanMouseMove>,
    mut windows: ResMut<Windows>,
    mut cursor: ResMut<Cursor>,
    state: Res<InterfaceState>,
    mut players: Query<&mut ActorFlags, With<HostPlayer>>,
) {
    let mut window = windows.primary_mut();

    let mut flags = players.single_mut();

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
    cursor.reset(window);
}

/// Create the death widget if the player dies.
pub fn death(mut state: ResMut<InterfaceState>, players: Query<&Death, With<HostPlayer>>) {
    if players.get_single().is_ok() {
        state.push(widgets::Death::default());
    }
}
