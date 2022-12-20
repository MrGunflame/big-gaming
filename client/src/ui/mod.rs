mod cursor;
mod events;
mod gun;
mod interfaces;
mod menu;
mod widgets;

pub mod crosshair;
pub mod debug;
pub mod health;

use std::borrow::Borrow;
use std::collections::HashMap;
use std::ptr::NonNull;

use bevy::prelude::{Component, Plugin, Resource};
use bevy_egui::EguiPlugin;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub enum Focus {
    /// The focus is on the game world.
    World,
    /// The focus is on an interface.
    Interface,
}

/// The user interface plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(EguiPlugin)
            .insert_resource(InterfaceState::new())
            .add_event::<Focus>()
            .add_startup_system(events::register_events)
            .add_system(events::handle_events)
            .add_system(events::toggle_focus)
            .add_system(crosshair::crosshair)
            .add_system(health::health)
            .add_system(debug::debug)
            .add_system(menu::gamemenu::gamemenu)
            .add_system(menu::death::death)
            .add_system(menu::inventory::inventory)
            .add_system(gun::gun);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InterfaceId(u32);

#[derive(Debug, Resource)]
pub struct InterfaceState {
    interfaces: HashMap<InterfaceId, Option<NonNull<()>>>,
    /// Does any interface capture mouse/keyboard inputs?
    capture: bool,
}

impl InterfaceState {
    pub fn new() -> Self {
        Self {
            interfaces: HashMap::new(),
            capture: false,
        }
    }

    pub fn contains<T>(&self, id: T) -> bool
    where
        T: Borrow<InterfaceId>,
    {
        self.interfaces.contains_key(id.borrow())
    }

    // FIXME: The `Sized` type requirement can be removed.
    pub fn insert<U>(&mut self, id: InterfaceId, data: Option<U>)
    where
        U: Send + Sync + 'static,
    {
        let ptr = data.map(|data| {
            let boxed = Box::new(data);

            // SAFETY: The pointer returned by `Box::into_raw` is always non-null.
            unsafe { NonNull::new_unchecked(Box::into_raw(boxed)).cast() }
        });

        self.interfaces.insert(id, ptr);
    }

    /// # Safety
    ///
    /// `U` must be the same type, or a type with the same ABI as `U`, as the type `U` when
    /// [`insert`] was called.
    ///
    /// [`insert`]: Self::insert
    pub unsafe fn remove<T, U>(&mut self, id: T) -> Option<Box<U>>
    where
        T: Borrow<InterfaceId>,
    {
        let ptr = self.interfaces.remove(id.borrow())?;

        match ptr {
            Some(ptr) => {
                // SAFETY: The caller guarantees that `U` has the same ABI as the
                // type behind `ptr`.
                Some(unsafe { Box::from_raw(ptr.cast().as_ptr()) })
            }
            None => None,
        }
    }

    /// Returns a reference to the data of the given [`InterfaceId`].
    ///
    /// # Safety
    ///
    /// `U` must be the same type, or a type with the same ABI as `U`, as the type `U` when
    /// [`insert`] was called.
    ///
    /// [`insert`]: Self::insert
    pub unsafe fn get<T, U>(&self, id: T) -> Option<&U>
    where
        T: Borrow<InterfaceId>,
        U: Send + Sync + 'static,
    {
        let ptr = self.interfaces.get(id.borrow())?;

        match ptr {
            Some(ptr) => {
                // SAFETY: The caller guarantees that `U` has the same ABI as the
                // type behind `ptr`.
                Some(unsafe { ptr.cast().as_ref() })
            }
            None => None,
        }
    }

    /// Returns the data of the given [`InterfaceId`].
    ///
    /// # Safety
    ///
    /// `U` must be the same type, or a type with the same ABI as `U`, as the type `U` when
    /// [`insert`] was called.
    ///
    /// [`insert`]: Self::insert
    pub unsafe fn get_mut<T, U>(&self, id: T) -> Option<&mut U>
    where
        T: Borrow<InterfaceId>,
        U: Send + Sync + 'static,
    {
        let ptr = self.interfaces.get(id.borrow())?;

        match ptr {
            Some(ptr) => {
                // SAFETY: The caller guarantees that `U` has the same ABI as the
                // type behind `ptr`.
                Some(unsafe { ptr.cast().as_mut() })
            }
            None => None,
        }
    }

    pub fn get_raw<T>(&self, id: T) -> Option<NonNull<()>>
    where
        T: Borrow<InterfaceId>,
    {
        let ptr = self.interfaces.get(id.borrow())?;
        *ptr
    }
}

// SAFETY: As long as the contained data is `Send + Sync` as required in the
// `insert` function signature, `InterfaceState` is also `Send + Sync`.
unsafe impl Send for InterfaceState {}
unsafe impl Sync for InterfaceState {}
