//! Hotkey handling plugin
//!
//! [`HotkeyStore`] provides an efficient and flexible resource for user-defined hotkeys. it it
//! optimized for fast read access.
use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::{EventReader, Input, KeyCode, Plugin, Res, ResMut, Resource};

static EVENT_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(HotkeyStore::new())
            .add_system(keyboard_input);
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub id: EventId,
    pub name: &'static str,
    pub trigger: Option<KeyCode>,
    pub state: bool,
}

impl Event {
    pub fn new() -> Self {
        Self {
            id: EventId::new(),
            name: "",
            trigger: None,
            state: false,
        }
    }

    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Into<KeyCode>,
    {
        self.trigger = Some(trigger.into());
        self
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct EventCell {
    cell: UnsafeCell<Event>,
}

impl EventCell {
    #[inline]
    const fn new(event: Event) -> Self {
        Self {
            cell: UnsafeCell::new(event),
        }
    }

    #[inline]
    unsafe fn get(&self) -> &Event {
        unsafe { &*self.cell.get() }
    }

    #[inline]
    unsafe fn get_mut(&self) -> &mut Event {
        unsafe { &mut *self.cell.get() }
    }
}

impl Borrow<EventId> for EventCell {
    fn borrow(&self) -> &EventId {
        unsafe { &self.get().id }
    }
}

impl Hash for EventCell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let event = unsafe { self.get() };

        event.id.hash(state);
    }
}

impl PartialEq for EventCell {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.get().id == other.get().id }
    }
}

impl PartialEq<EventId> for EventCell {
    fn eq(&self, other: &EventId) -> bool {
        unsafe { self.get().id == *other }
    }
}

impl Eq for EventCell {}

unsafe impl Send for EventCell {}
unsafe impl Sync for EventCell {}

/// A unique identifier for an input event.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventId(NonZeroU32);

impl EventId {
    pub fn new() -> Self {
        let id = EVENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        if id == u32::MAX {
            panic!("Exceeded the maximum number of EventIds");
        }

        debug_assert!(id != 0);

        unsafe { Self(NonZeroU32::new_unchecked(id)) }
    }
}

#[derive(Debug, Resource)]
pub struct HotkeyStore {
    keyboard: HashSet<KeyCode>,
    events: HashSet<EventCell>,
}

impl HotkeyStore {
    pub fn new() -> Self {
        Self {
            keyboard: HashSet::new(),
            events: HashSet::new(),
        }
    }

    /// Registers a new event.
    pub fn register(&mut self, event: Event) -> EventId {
        let id = event.id;
        self.events.insert(EventCell::new(event));
        id
    }

    /// Returns `true` if an [`Event`] is triggered.
    pub fn triggered<T>(&self, id: T) -> bool
    where
        T: Borrow<EventId>,
    {
        match self.events.get(id.borrow()) {
            Some(event) => {
                let event = unsafe { event.get() };
                event.state
            }
            None => false,
        }
    }
}

fn keyboard_input(mut hotkeys: ResMut<HotkeyStore>, mut events: EventReader<KeyboardInput>) {
    hotkeys.keyboard.clear();

    for event in events.iter() {
        if let Some(key_code) = event.key_code {
            hotkeys.keyboard.insert(key_code);
        }
    }

    for event in hotkeys.events.iter() {
        let event = unsafe { event.get_mut() };

        if let Some(trigger) = event.trigger {
            event.state = hotkeys.keyboard.get(&trigger).is_some();
        }
    }
}
