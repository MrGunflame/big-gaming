use std::mem::MaybeUninit;

use bevy::prelude::{EventReader, EventWriter, KeyCode, Query, Res, ResMut};
use bevy::window::Windows;

use crate::plugins::hotkeys::{Event, EventId, HotkeyStore, TriggerKind};

use super::cursor::Cursor;
use super::debug::Debug;
use super::menu::console::Console;
use super::menu::gamemenu::GameMenu;
use super::menu::inventory::InventoryMenu;
use super::{Focus, InterfaceState};

const DEFAULT_TRIGGER_GAMEMENU: KeyCode = KeyCode::Escape;
const DEFAULT_TRIGGER_DEBUGMENU: KeyCode = KeyCode::F3;
const DEFAULT_TRIGGER_INVENTORY: KeyCode = KeyCode::I;
const DEFAULT_TRIGGER_CONSOLE: KeyCode = KeyCode::Return;

static mut EVENTS: MaybeUninit<Events> = MaybeUninit::uninit();

struct Events {
    game_menu: EventId,
    debug_menu: EventId,
    inventory: EventId,
    console: EventId,
}

pub(super) fn register_events(mut hotkeys: ResMut<HotkeyStore>) {
    let game_menu = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_GAMEMENU)
            .kind(TriggerKind::Trigger),
    );

    let debug_menu = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_DEBUGMENU)
            .kind(TriggerKind::Trigger),
    );

    let inventory = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_INVENTORY)
            .kind(TriggerKind::Trigger),
    );

    let console = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_CONSOLE)
            .kind(TriggerKind::Trigger),
    );

    unsafe {
        EVENTS.write(Events {
            game_menu,
            debug_menu,
            inventory,
            console,
        });
    }
}

pub(super) fn handle_events(
    hotkeys: Res<HotkeyStore>,
    mut state: ResMut<InterfaceState>,
    mut focus: EventWriter<Focus>,
) {
    let events = unsafe { EVENTS.assume_init_ref() };

    let previous = state.is_empty();

    if hotkeys.triggered(events.game_menu) {
        if state.is_empty() {
            state.push_default::<GameMenu>();
        } else {
            let _ = state.pop();
        }
    }

    if hotkeys.triggered(events.debug_menu) {
        if state.remove::<Debug>().is_none() {
            state.push_default::<Debug>();
        }
    }

    if hotkeys.triggered(events.inventory) {
        if state.remove::<InventoryMenu>().is_none() {
            state.push_default::<InventoryMenu>();
        }
    }

    if hotkeys.triggered(events.console) {
        if state.remove::<Console>().is_none() {
            state.push_default::<Console>();
        }
    }

    if previous != state.is_empty() {
        if state.is_empty() {
            focus.send(Focus::World);
        } else {
            focus.send(Focus::Interface);
        }
    }
}

/// Toggle [`Focus`].
pub(super) fn toggle_focus(
    mut windows: ResMut<Windows>,
    mut players: Query<&mut Focus>,
    mut events: EventReader<Focus>,
) {
    let window = windows.primary_mut();
    let mut focus = players.single_mut();

    for event in events.iter() {
        *focus = *event;

        match event {
            Focus::World => Cursor::lock(window),
            Focus::Interface => Cursor::unlock(window),
        }
    }
}
