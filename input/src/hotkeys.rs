//! Hotkey handling
//!
//!

use std::borrow::{Borrow, Cow};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::input::ButtonState;
use bevy::prelude::{EventReader, EventWriter, KeyCode, Plugin, Res, ResMut, Resource, ScanCode};
use bevy_ecs::system::SystemParam;

use crate::keyboard::KeyboardInput;

static EVENT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Hotkeys::new())
            .add_event::<Hotkey>()
            .add_system(keyboard_input)
            .add_system(send_hotkey_events);
    }
}

/// The global registry managing hotkeys.
#[derive(Debug, Resource)]
pub struct Hotkeys {
    inputs: InputMap,
    hotkeys: HotkeyMap,
}

impl Hotkeys {
    /// Creates a new `Hotkeys`.
    pub fn new() -> Self {
        Self {
            inputs: InputMap::new(),
            hotkeys: HotkeyMap::new(),
        }
    }

    /// Registers a new [`Hotkey`]. Returns the [`HotkeyId`] assigned to the registered [`Hotkey`].
    ///
    /// The returned [`HotkeyId`] can be used to filter hotkey events to only the registered
    /// hotkey.
    pub fn register<T>(&mut self, hotkey: T) -> HotkeyId
    where
        T: Into<Hotkey>,
    {
        let id = EVENT_ID.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            panic!("Overflown HotkeyId");
        }

        let mut hotkey = hotkey.into();
        hotkey.id = HotkeyId(id);

        self.hotkeys.insert(hotkey);

        HotkeyId(id)
    }

    /// Unregisters and returns the [`Hotkey`] with the given `id`. Returns `None` if no [`Hotkey`]
    /// with the given `id` was registered.
    pub fn unregister<T>(&mut self, id: T) -> Option<Hotkey>
    where
        T: Borrow<HotkeyId>,
    {
        self.hotkeys.remove(id)
    }
}

#[derive(Debug)]
struct HotkeyMap {
    hotkeys: Vec<(Hotkey, HotkeyState)>,
    ids: HashMap<HotkeyId, usize>,
    keys: HashMap<HotkeyCode, Vec<usize>>,
}

impl HotkeyMap {
    pub fn new() -> Self {
        Self {
            hotkeys: Vec::new(),
            ids: HashMap::new(),
            keys: HashMap::new(),
        }
    }

    /// Inserts a new [`Hotkey`] into the `HotkeyMap`.
    ///
    /// Note that the [`HotkeyId`] should be set to unique value before calling this function.
    /// `insert` will overwrite the existing [`Hotkey`] with the same `id` if one exists.
    pub fn insert(&mut self, hotkey: Hotkey) {
        let state = HotkeyState::new(&hotkey);

        self.hotkeys.push((hotkey, state));
        self.rebuild();
    }

    /// Removes and returns a [`Hotkey`] with the given `id`. Returns `None` if no [`Hotkey`] with
    /// the given `id` exists.
    pub fn remove<T>(&mut self, id: T) -> Option<Hotkey>
    where
        T: Borrow<HotkeyId>,
    {
        let index = self.ids.remove(id.borrow())?;

        // FIXME: The bounds check can be avoided.
        let (hotkey, _) = self.hotkeys.remove(index);

        // The indices have changed so a rebuild is necessary.
        self.rebuild();

        Some(hotkey)
    }

    /// Rebuild the internal maps.
    fn rebuild(&mut self) {
        self.ids.clear();
        self.keys.clear();

        for (index, (hotkey, _)) in self.hotkeys.iter().enumerate() {
            self.ids.insert(hotkey.id, index);

            let indices = self.keys.entry(hotkey.default.code).or_insert(Vec::new());
            indices.push(index);
        }
    }

    /// Clears the inputs from the last tick.
    fn clear(&mut self) {
        for (_, state) in self.hotkeys.iter_mut() {
            state.clear();
        }
    }

    /// Signals that `key` was *just pressed*.
    ///
    /// This should only be called *once* when a key is first pressed. It should **not** be called
    /// continuously.
    fn press(&mut self, key: HotkeyCode) {
        let Some(hotkeys) = self.keys.get(&key) else {
            return;
        };

        for index in hotkeys {
            let (_, state) = &mut self.hotkeys[*index];
            state.press(key);
        }
    }

    /// Signals that `key` was *just released*.
    ///
    /// This should only be called *once* when a key is first released. It should **not** be called
    /// continuously.
    fn release(&mut self, key: HotkeyCode) {
        let Some(hotkeys) = self.keys.get(&key) else {
            return;
        };

        for index in hotkeys {
            let (_, state) = &mut self.hotkeys[*index];
            state.release(key);
        }
    }

    /// Returns an iterator over all [`HotkeyState`]s in this `HotkeyMap`.
    #[inline]
    fn states(&self) -> States<'_> {
        States {
            inner: self,
            next: 0,
        }
    }
}

/// An iterator over all [`HotkeyState`]s in a [`HotkeyMap`].
///
/// Returned by [`states`].
///
/// [`states`]: HotkeyMap::states
#[derive(Clone, Debug)]
struct States<'a> {
    inner: &'a HotkeyMap,
    next: usize,
}

impl<'a> Iterator for States<'a> {
    type Item = &'a HotkeyState;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let elem = self.inner.hotkeys.get(self.next)?;
        self.next += 1;
        Some(&elem.1)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for States<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.hotkeys.len() - self.next
    }
}

impl<'a> FusedIterator for States<'a> {}

#[derive(Debug)]
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

/// The current state of a [`Hotkey`].
///
/// Since [`Hotkey`]s may include multiple key, it is not sufficient to use a simple `bool`.
#[derive(Clone, Debug)]
struct HotkeyState {
    states: Box<[(Key, bool)]>,
}

impl HotkeyState {
    fn new(hotkey: &Hotkey) -> Self {
        let mut states = Vec::with_capacity(1);
        states.push((hotkey.default, false));

        Self {
            states: states.into_boxed_slice(),
        }
    }

    fn is_active(&self) -> bool {
        for (_, state) in self.states.iter() {
            if !state {
                return false;
            }
        }

        true
    }

    fn clear(&mut self) {
        for (code, state) in self.states.iter_mut() {
            if matches!(
                code.trigger,
                TriggerKind::JustPressed | TriggerKind::JustReleased
            ) {
                *state = false;
            }
        }
    }

    fn press(&mut self, key: HotkeyCode) {
        for (code, state) in self.states.iter_mut() {
            if code.code != key {
                continue;
            }

            if matches!(
                code.trigger,
                TriggerKind::Pressed | TriggerKind::JustPressed
            ) {
                *state = true;
            }
        }
    }

    fn release(&mut self, key: HotkeyCode) {
        for (code, state) in self.states.iter_mut() {
            if code.code != key {
                continue;
            }

            if matches!(code.trigger, TriggerKind::JustReleased) {
                *state = true;
            } else {
                *state = false;
            }
        }
    }
}

#[derive(SystemParam)]
pub struct HotkeyReader<'w, 's, H>
where
    H: HotkeyFilter,
{
    reader: EventReader<'w, 's, Hotkey>,
    #[system_param(ignore)]
    _marker: PhantomData<&'static H>,
}

impl<'w, 's, H> HotkeyReader<'w, 's, H>
where
    H: HotkeyFilter,
{
    pub fn iter(&mut self) -> impl Iterator<Item = &Hotkey> {
        self.reader.iter().filter(|event| H::filter(event.id))
    }
}

// pub struct Iter {}

pub trait HotkeyFilter: Send + Sync + 'static {
    /// Returns `true` if the [`HotkeyId`] should be yielded.
    fn filter(id: HotkeyId) -> bool;
}

#[derive(Clone, Debug)]
pub struct Hotkey {
    pub id: HotkeyId,
    pub name: Cow<'static, str>,
    pub default: Key,
}

impl HotkeyFilter for Hotkey {
    #[inline]
    fn filter(_: HotkeyId) -> bool {
        true
    }
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

// TOOD: Better name? HotkeyDescriptor?
// FIXME: TriggerKind maybe should be included here to allow
// customization of modifier key triggers.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HotkeyCode {
    KeyCode { key_code: KeyCode },
    ScanCode { scan_code: ScanCode },
}

#[derive(Copy, Clone, Debug)]
pub struct Key {
    pub trigger: TriggerKind,
    pub code: HotkeyCode,
}

#[derive(Copy, Clone, Debug)]
pub enum HotkeyKind {
    KeyCode { key_code: KeyCode },
    ScanCode { scan_code: ScanCode },
}

fn keyboard_input(mut hotkeys: ResMut<Hotkeys>, mut events: EventReader<KeyboardInput>) {
    hotkeys.hotkeys.clear();

    for event in events.iter() {
        let Some(key_code) = event.key_code else {
            continue;
        };

        match event.state {
            ButtonState::Pressed => hotkeys.hotkeys.press(HotkeyCode::KeyCode { key_code }),
            ButtonState::Released => hotkeys.hotkeys.release(HotkeyCode::KeyCode { key_code }),
        }
    }
}

fn send_hotkey_events(hotkeys: Res<Hotkeys>, mut writer: EventWriter<Hotkey>) {
    dbg!(&hotkeys);

    for (hotkey, state) in &hotkeys.hotkeys.hotkeys {
        // dbg!(hotkey, state);

        if state.is_active() {
            writer.send(hotkey.clone());
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

    use super::{HotkeyFilter, HotkeyId, HotkeyReader};

    struct Inventory;

    impl HotkeyFilter for Inventory {
        fn filter(id: HotkeyId) -> bool {
            true
        }
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
