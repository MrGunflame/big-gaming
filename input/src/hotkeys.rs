use std::borrow::{Borrow, Cow};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

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

    pub fn register<T>(&self, hotkey: T) -> HotkeyId
    where
        T: Into<Hotkey>,
    {
        let id = EVENT_ID.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            panic!("Overflown HotkeyId");
        }

        HotkeyId(id)
    }
}

/// A `HotkeyMap` may be indexed by both [`HotkeyId`] and [`ScanCode`].
struct HotkeyMap {
    hotkeys: Vec<Hotkey>,
    ids: HashMap<HotkeyId, usize>,
    scan_codes: HashMap<ScanCode, Vec<usize>>,
}

impl HotkeyMap {
    pub fn new() -> Self {
        Self {
            hotkeys: Vec::new(),
            ids: HashMap::new(),
            scan_codes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, hotkey: Hotkey) {
        if self.hotkeys.len() == self.hotkeys.capacity() {
            self.insert_slow(hotkey);
        } else {
            unsafe {
                self.insert_fast(hotkey);
            }
        }
    }

    pub fn remove<T>(&mut self, id: T) -> Option<Hotkey>
    where
        T: Borrow<HotkeyId>,
    {
        let index = self.ids.remove(id.borrow())?;

        // FIXME: The bounds check can be avoided.
        Some(self.hotkeys.remove(index))
    }

    pub fn get_by_id(&self, id: HotkeyId) -> Option<&Hotkey> {
        let index = *self.ids.get(&id)?;

        Some(unsafe { self.hotkeys.get_unchecked(index) })
    }

    #[inline]
    unsafe fn insert_fast(&mut self, hotkey: Hotkey) {
        let id = hotkey.id;

        let index = self.hotkeys.len();
        self.hotkeys.push(hotkey);

        self.ids.insert(id, index);
    }

    fn insert_slow(&mut self, hotkey: Hotkey) {
        self.ids.clear();

        self.hotkeys.push(hotkey);

        for (index, hotkey) in self.hotkeys.iter().enumerate() {
            self.ids.insert(hotkey.id, index);
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
pub struct HotkeyReader<'w, 's, H>
where
    H: AsHotkey,
{
    reader: EventReader<'w, 's, Hotkey>,
    #[system_param(ignore)]
    _marker: PhantomData<&'static H>,
}

impl<'w, 's, H> HotkeyReader<'w, 's, H>
where
    H: AsHotkey,
{
    pub fn iter(&mut self) -> impl Iterator<Item = &Hotkey> {
        self.reader.iter().filter(|event| event.id == H::ID)
    }
}

// pub struct Iter {}

/// A [`Hotkey`] registered at compile time.
pub trait AsHotkey: Send + Sync + 'static {
    const ID: HotkeyId;
}

#[derive(Clone, Debug)]
pub struct Hotkey {
    pub id: HotkeyId,
    pub name: Cow<'static, str>,
    pub trigger: TriggerKind,
    pub default: HotkeyKind,
}

impl AsRef<HotkeyId> for Hotkey {
    #[inline]
    fn as_ref(&self) -> &HotkeyId {
        &self.id
    }
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
#[repr(transparent)]
pub struct HotkeyId(pub u32);

#[cfg(test)]
mod tests {
    use bevy_ecs::schedule::{Schedule, StageLabel, SystemStage};
    use bevy_ecs::system::{ResMut, Resource};
    use bevy_ecs::world::World;

    use super::{AsHotkey, HotkeyId, HotkeyReader};

    struct Inventory;

    impl AsHotkey for Inventory {
        const ID: super::HotkeyId = HotkeyId(0);
    }

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
