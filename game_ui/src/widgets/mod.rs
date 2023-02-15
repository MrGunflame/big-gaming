mod console;
mod crosshair;
mod death;
mod debug;
mod gamemenu;
mod health;
mod inventory;
mod weapon;

use std::borrow::Cow;

use bevy::prelude::{App, Input, KeyCode, Res, ResMut};
pub use console::Console;
pub use crosshair::Crosshair;
pub use death::Death;
pub use debug::DebugInfo;
pub use gamemenu::GameMenu;
pub use health::Health;
pub use inventory::Inventory;
pub use weapon::Weapon;

use game_input::hotkeys::{
    Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind,
};

use crate::InterfaceState;

static mut INVENTORY: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("inventory"),
    default: Key {
        trigger: TriggerKind::JUST_PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::I,
        },
    },
};

static mut CONSOLE: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("console"),
    default: Key {
        trigger: TriggerKind::JUST_PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::Return,
        },
    },
};

static mut DEBUG: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("debug"),
    default: Key {
        trigger: TriggerKind::JUST_PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::F3,
        },
    },
};

struct InventoryHotkey;

impl HotkeyFilter for InventoryHotkey {
    fn filter(id: HotkeyId) -> bool {
        let want = unsafe { &INVENTORY }.id;
        want == id
    }
}

struct ConsoleHotkey;

impl HotkeyFilter for ConsoleHotkey {
    fn filter(id: HotkeyId) -> bool {
        let want = unsafe { &CONSOLE }.id;
        want == id
    }
}

struct DebugHotkey;

impl HotkeyFilter for DebugHotkey {
    fn filter(id: HotkeyId) -> bool {
        let want = unsafe { &DEBUG }.id;
        want == id
    }
}

pub(super) fn register_hotkeys(mut hotkeys: ResMut<Hotkeys>) {
    let mut inventory = unsafe { &mut INVENTORY };
    let id = hotkeys.register(inventory.clone());
    inventory.id = id;
    drop(inventory);

    let mut console = unsafe { &mut CONSOLE };
    let id = hotkeys.register(console.clone());
    console.id = id;
    drop(console);

    let mut debug = unsafe { &mut DEBUG };
    let id = hotkeys.register(debug.clone());
    debug.id = id;
    drop(debug);
}

pub(super) fn register_hotkey_systems(app: &mut App) {
    app.add_system(escape)
        .add_system(toggle_inventory)
        .add_system(toggle_console)
        .add_system(toggle_debug);
}

fn escape(mut state: ResMut<InterfaceState>, inputs: Res<Input<KeyCode>>) {
    if !inputs.just_pressed(KeyCode::Escape) {
        return;
    }

    if !state.pop() {
        state.push(GameMenu::default());
    }
}

fn toggle_inventory(mut state: ResMut<InterfaceState>, mut events: HotkeyReader<InventoryHotkey>) {
    for _ in events.iter() {
        state.push(Inventory::default());
    }
}

fn toggle_console(mut state: ResMut<InterfaceState>, mut events: HotkeyReader<ConsoleHotkey>) {
    for _ in events.iter() {
        state.push(Console::default());
    }
}

fn toggle_debug(mut state: ResMut<InterfaceState>, mut events: HotkeyReader<DebugHotkey>) {
    for _ in events.iter() {
        state.push(DebugInfo::default());
    }
}
