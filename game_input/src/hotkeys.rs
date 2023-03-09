//! Dynamic input hotkey handling
//!
//! # What is a *Hotkey*?
//!
//! A *Hotkey* is a action triggered by a user input. Hotkeys provide additional capabilites that
//! make them work better than simply listening for events from an input device.
//!
//! - A hotkey allows rebinding of the hotkey by the user.
//! - A hotkey allows any number of combined inputs of any input device.
//! - A hotkey allows to check for changes on a hotkey.
//!
//! # Hotkey triggers
//!
//! Every [`Hotkey`] comes with at least one [`TriggerKind`] condition. Whenever an input occurs
//! that triggers a defined [`TriggerKind`] a [`Event`] is dispatched at to all listeners. When
//! multiple [`TriggerKind`]s are defined on a single [`Hotkey`], each input that triggers a
//! [`TriggerKind`] dispatches its own [`Event`].
//!
//! Note that when a [`Hotkey`] was registered on multiple inputs, the triggers are still treated
//! as one. This means that `JUST_PRESSED` triggers when the combination of inputs was first
//! pressed, in any order. `JUST_RELEASE` triggers when any input from the combination was
//! released.
//!
//! # Hotkey rebinding
//!
//! Registered [`Hotkey`]s define a default input. The input sequence of the [`Hotkey`] may be
//! changed dynamically at runtime. Multiple nputs across input devices are allowed.
//!
//! # The Escape key
//!
//! The escape key ([`KeyCode::Escape`]) is not allowed in any [`Hotkey`]. If you register one on
//! said key (or associated [`ScanCode`]) you will never see it trigger.
//!
//! The escape key is hardcoded to access the game menu or close UI widgets. This behavior is on
//! purpose as the `Escape` key is typically used as a "escape" action. It can purposefully not be
//! assigned to a [`Hotkey`] to prevent unintuitive behavior.
//!
//! If it is absolutely necessary the access the `Escape` key, it can still be accessed via the
//! lower-level [`Input`] resource, thought it is heavily discouraged to do so.
//!
//! # "Best practice" usage notes
//!
//! To make use of the full feature set that the hotkey library provides, it is recommended to
//! follow a few guidelines.
//!
//! - **Do not register any [`Hotkey`]s on the `Escape` key.** It will never trigger.
//! - Register multiple [`Hotkey`]s for each action instead of re-using the same [`Hotkey`] for
//! multiple actions depending on the context.
//! - Don't register multiple [`Hotkey`]s for "start-stop" style events (events with an
//! `JUST_PRESSED` and `JUST_RELEASED` trigger, but no `PRESSED` trigger). Instead register a
//! single [`Hotkey`] with both the `JUST_PRESSED` and `JUST_RELEASED` triggers.
//!

use std::borrow::{Borrow, Cow};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::input::ButtonState;
use bevy::prelude::{
    EventReader, EventWriter, IntoSystemConfig, IntoSystemSetConfig, KeyCode, MouseButton, Plugin,
    Res, ResMut, Resource, ScanCode, SystemSet,
};
use bevy_ecs::system::SystemParam;

use crate::keyboard::KeyboardInput;
use crate::mouse::MouseButtonInput;

static EVENT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum HotkeySet {
    Reset,
    Update,
    Dispatch,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Hotkeys::new());
        app.add_event::<Event>();

        // Reset
        app.add_system(reset_hotkeys.in_set(HotkeySet::Reset));

        // Update
        app.add_system(keyboard_input.in_set(HotkeySet::Update));
        app.add_system(mouse_input.in_set(HotkeySet::Update));

        // Dispatch
        app.add_system(send_hotkey_events.in_set(HotkeySet::Dispatch));

        app.configure_set(HotkeySet::Reset.before(HotkeySet::Update));
        app.configure_set(HotkeySet::Update.before(HotkeySet::Dispatch));
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
    trigger: TriggerKind,
    states: HashMap<HotkeyCode, bool>,
    just_pressed: bool,
    just_released: bool,
}

impl HotkeyState {
    /// Returns a new `HotkeyState` from the given [`Hotkey`].
    fn new(hotkey: &Hotkey) -> Self {
        let mut states = HashMap::with_capacity(1);
        states.insert(hotkey.default.code, false);

        Self {
            trigger: hotkey.default.trigger,
            states,
            just_pressed: false,
            just_released: false,
        }
    }

    fn get(&self) -> Option<TriggerKind> {
        if self.trigger.intersects(TriggerKind::JUST_PRESSED) && self.just_pressed {
            return Some(TriggerKind::JUST_PRESSED);
        }

        if self.trigger.intersects(TriggerKind::JUST_RELEASED) && self.just_released {
            return Some(TriggerKind::JUST_RELEASED);
        }

        if self.trigger.intersects(TriggerKind::PRESSED) {
            for (_, state) in self.states.iter() {
                if !state {
                    return None;
                }
            }

            return Some(TriggerKind::PRESSED);
        }

        None
    }

    fn clear(&mut self) {
        self.just_pressed = false;
        self.just_released = false;
    }

    fn press(&mut self, key: HotkeyCode) {
        let Some(state) = self.states.get_mut(&key) else {
            return;
        };

        if !*state {
            *state = true;

            for state in self.states.values() {
                if !state {
                    return;
                }
            }

            self.just_pressed = true;
        }
    }

    fn release(&mut self, key: HotkeyCode) {
        let mut is_pressed = true;
        for state in self.states.values() {
            if !state {
                is_pressed = false;
                break;
            }
        }

        let Some(state)  = self.states.get_mut(&key) else {
            return;
        };

        *state = false;

        if is_pressed {
            self.just_released = true;
        }
    }
}

#[derive(SystemParam)]
pub struct HotkeyReader<'w, 's, H>
where
    H: HotkeyFilter,
{
    reader: EventReader<'w, 's, Event>,
    #[system_param(ignore)]
    _marker: PhantomData<&'static H>,
}

impl<'w, 's, H> HotkeyReader<'w, 's, H>
where
    H: HotkeyFilter,
{
    pub fn iter(&mut self) -> impl Iterator<Item = &Event> {
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

impl Hotkey {
    /// Creates a new `HotkeyBuilder`.
    #[inline]
    pub const fn builder() -> HotkeyBuilder {
        HotkeyBuilder::new()
    }
}

/// A builder for a [`Hotkey`].
#[derive(Clone, Debug)]
pub struct HotkeyBuilder {
    inner: Hotkey,
}

impl HotkeyBuilder {
    /// Creates a new `HotkeyBuilder`.
    pub const fn new() -> Self {
        Self {
            inner: Hotkey {
                id: HotkeyId(0),
                name: Cow::Borrowed("<unknown>"),
                default: Key {
                    trigger: TriggerKind::JUST_PRESSED,
                    code: HotkeyCode::KeyCode {
                        key_code: KeyCode::Escape,
                    },
                },
            },
        }
    }

    /// Sets the name of the [`Hotkey`].
    #[inline]
    pub fn name<T>(mut self, name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.inner.name = name.into();
        self
    }

    /// Sets the trigger for the [`Hotkey`].
    #[inline]
    pub fn trigger(mut self, trigger: TriggerKind) -> Self {
        self.inner.default.trigger = trigger;
        self
    }

    /// Sets the input for the [`Hotkey`].
    #[inline]
    pub fn input<T>(mut self, input: T) -> Self
    where
        T: Into<HotkeyCode>,
    {
        self.inner.default.code = input.into();
        self
    }

    /// Consumes this `HotkeyBuilder` returning the constructed [`Hotkey`].
    #[inline]
    pub fn build(self) -> Hotkey {
        self.inner
    }
}

impl Default for HotkeyBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<HotkeyBuilder> for Hotkey {
    #[inline]
    fn from(value: HotkeyBuilder) -> Self {
        value.build()
    }
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

/// What triggers should a [`Hotkey`] react to.
///
/// `TriggerKind` can be combined using the [`BitOr`] implementation:
///
/// ```
/// # use game_input::hotkeys::TriggerKind;
/// #
/// let just_pressed = TriggerKind::JUST_PRESSED;
/// let just_released = TriggerKind::JUST_RELEASED;
///
/// let just_pressed_or_released = just_pressed | just_released;
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TriggerKind(u8);

impl TriggerKind {
    /// A trigger that corresponds to no action.
    ///
    /// Note that `NONE` is never sent by an [`Event`].
    pub const NONE: Self = Self(0);

    /// Triggers an action **while** a hotkey is pressed.
    pub const PRESSED: Self = Self(1);

    /// Triggers an action when a hotkey is first pressed.
    pub const JUST_PRESSED: Self = Self(1 << 1);

    /// Triggers an action when the hotkey is released.
    pub const JUST_RELEASED: Self = Self(1 << 2);

    /// Returns `true` if `self` contains [`PRESSED`].
    ///
    /// [`PRESSED`]: Self::PRESSED
    #[inline]
    pub fn pressed(self) -> bool {
        self & Self::PRESSED != Self::NONE
    }

    /// Returns `true` if `self` contains [`JUST_PRESSED`].
    ///
    /// [`JUST_PRESSED`]: Self::JUST_PRESSED
    #[inline]
    pub fn just_pressed(self) -> bool {
        self & Self::JUST_PRESSED != Self::NONE
    }

    /// Returns `true` if `self` contains [`JUST_RELEASED`].
    ///
    /// [`JUST_RELEASED`]: Self::JUST_RELEASED
    #[inline]
    pub fn just_released(self) -> bool {
        self & Self::JUST_RELEASED != Self::NONE
    }

    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self & other != Self::NONE
    }
}

impl const BitOr for TriggerKind {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for TriggerKind {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl const BitAnd for TriggerKind {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for TriggerKind {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// An event dispatched when a [`Hotkey`] was activated.
#[derive(Copy, Clone, Debug)]
pub struct Event {
    /// The id of the [`Hotkey`] that triggered this `Event`.
    pub id: HotkeyId,
    /// The action that triggered the [`Hotkey`].
    pub trigger: TriggerKind,
}

// TOOD: Better name? HotkeyDescriptor?
// FIXME: TriggerKind maybe should be included here to allow
// customization of modifier key triggers.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HotkeyCode {
    KeyCode { key_code: KeyCode },
    ScanCode { scan_code: ScanCode },
    MouseButton { button: MouseButton },
}

impl From<KeyCode> for HotkeyCode {
    #[inline]
    fn from(value: KeyCode) -> Self {
        Self::KeyCode { key_code: value }
    }
}

impl From<ScanCode> for HotkeyCode {
    #[inline]
    fn from(value: ScanCode) -> Self {
        Self::ScanCode { scan_code: value }
    }
}

impl From<MouseButton> for HotkeyCode {
    #[inline]
    fn from(value: MouseButton) -> Self {
        Self::MouseButton { button: value }
    }
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
    MouseButton { button: MouseButton },
}

fn reset_hotkeys(mut hotkeys: ResMut<Hotkeys>) {
    hotkeys.hotkeys.clear();
}

fn keyboard_input(mut hotkeys: ResMut<Hotkeys>, mut events: EventReader<KeyboardInput>) {
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

fn mouse_input(mut hotkeys: ResMut<Hotkeys>, mut events: EventReader<MouseButtonInput>) {
    for event in events.iter() {
        match event.state {
            ButtonState::Pressed => hotkeys.hotkeys.press(HotkeyCode::MouseButton {
                button: event.button,
            }),
            ButtonState::Released => hotkeys.hotkeys.release(HotkeyCode::MouseButton {
                button: event.button,
            }),
        }
    }
}

fn send_hotkey_events(hotkeys: Res<Hotkeys>, mut writer: EventWriter<Event>) {
    for (hotkey, state) in &hotkeys.hotkeys.hotkeys {
        if let Some(trigger) = state.get() {
            writer.send(Event {
                id: hotkey.id,
                trigger,
            });
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HotkeyId(pub u32);

#[cfg(test)]
mod tests {
    use super::{Hotkey, HotkeyMap, TriggerKind};
    use crate::keyboard::KeyCode;

    #[test]
    fn test_hotkeymap() {
        let mut hotkeys = HotkeyMap::new();
        hotkeys.insert(
            Hotkey::builder()
                .trigger(TriggerKind::JUST_PRESSED)
                .input(KeyCode::Space)
                .build(),
        );

        let hotkey = hotkeys.states().nth(0).unwrap();
        assert!(hotkey.get().is_none());

        hotkeys.press(KeyCode::Space.into());
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), Some(TriggerKind::JUST_PRESSED));

        hotkeys.clear();
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert!(hotkey.get().is_none());

        hotkeys.release(KeyCode::Space.into());
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert!(hotkey.get().is_none());

        hotkeys.clear();
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert!(hotkey.get().is_none());

        hotkeys.press(KeyCode::Space.into());
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), Some(TriggerKind::JUST_PRESSED));
    }

    #[test]
    fn test_hotkeys_multi() {
        let mut hotkeys = HotkeyMap::new();
        hotkeys.insert(
            Hotkey::builder()
                .trigger(TriggerKind::JUST_PRESSED | TriggerKind::JUST_RELEASED)
                .input(KeyCode::Space)
                .build(),
        );

        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), None);

        hotkeys.press(KeyCode::Space.into());
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), Some(TriggerKind::JUST_PRESSED));

        hotkeys.clear();
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), None);

        hotkeys.release(KeyCode::Space.into());
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), Some(TriggerKind::JUST_RELEASED));

        hotkeys.clear();
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), None);

        hotkeys.press(KeyCode::Space.into());
        let hotkey = hotkeys.states().nth(0).unwrap();
        assert_eq!(hotkey.get(), Some(TriggerKind::JUST_PRESSED));
    }
}
