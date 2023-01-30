use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::atomic::AtomicU32;

use bevy::input::ButtonState;
use bevy::prelude::{EventReader, KeyCode, Plugin, ResMut, Resource, ScanCode};
use bevy_ecs::system::SystemParam;

use crate::keyboard::KeyboardInput;

static EVENT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Hotkeys::new())
            .add_system(keyboard_input);
    }
}

#[derive(Resource)]
pub struct Hotkeys {
    inputs: InputMap,
}

impl Hotkeys {
    pub fn new() -> Self {
        Self {
            inputs: InputMap::new(),
        }
    }
}

struct InputMap {
    pressed: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
}

impl InputMap {
    fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }

    fn press(&mut self, key: KeyCode) {
        self.pressed.insert(key);
        self.just_pressed.insert(key);
    }

    fn release(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
        self.just_released.insert(key);
    }

    fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

#[derive(SystemParam)]
pub struct HotkeyReader<'w, 's, E>
where
    E: Send + Sync + 'static,
{
    reader: EventReader<'w, 's, E>,
}

impl<'w, 's, E> HotkeyReader<'w, 's, E>
where
    E: Send + Sync + 'static,
{
    // pub fn iter(&self) -> Iter {
    //     Iter {}
    // }
    pub fn iter(&mut self) -> impl Iterator<Item = &E> {
        self.reader.iter()
    }
}

// pub struct Iter {}

pub trait IntoHotkey {
    fn into_hotkey(self) -> Hotkey;
}

#[derive(Clone, Debug)]
pub struct Hotkey {
    pub name: Cow<'static, str>,
    pub trigger: TriggerKind,
    pub default: HotkeyKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TriggerKind {
    Pressed,
    JustPressed,
    JustReleased,
}

#[derive(Copy, Clone, Debug)]
pub enum HotkeyKind {
    KeyCode { key_code: KeyCode },
    ScanCode { scan_code: ScanCode },
}

fn keyboard_input(mut hotkeys: ResMut<Hotkeys>, mut events: EventReader<KeyboardInput>) {
    hotkeys.inputs.clear();

    for event in events.iter() {
        let Some(key_code) = event.key_code else {
            continue;
        };

        match event.state {
            ButtonState::Pressed => hotkeys.inputs.press(key_code),
            ButtonState::Released => hotkeys.inputs.release(key_code),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct EventId(u32);

#[cfg(test)]
mod tests {
    use bevy_ecs::schedule::{Schedule, StageLabel, SystemStage};
    use bevy_ecs::system::{ResMut, Resource};
    use bevy_ecs::world::World;

    use super::HotkeyReader;

    struct Inventory;

    #[derive(Resource)]
    struct State(bool);

    #[derive(StageLabel)]
    struct TestStage;

    #[test]
    fn test_hotkeys() {
        let mut world = World::new();

        let mut schedule = Schedule::default();
        schedule.add_stage(
            TestStage,
            SystemStage::parallel().with_system(toggle_inventory),
        );

        schedule.run_once(&mut world);
    }

    fn toggle_inventory(mut state: ResMut<State>, mut events: HotkeyReader<Inventory>) {
        for _ in events.iter() {
            state.0 ^= true;
        }
    }
}
