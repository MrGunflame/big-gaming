mod events;
mod interfaces;
mod menu;
mod widgets;

pub mod crosshair;
pub mod debug;
pub mod health;

use std::collections::HashSet;

use bevy::prelude::{Plugin, Resource};
use bevy_egui::EguiPlugin;

/// The user interface plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(EguiPlugin)
            .insert_resource(InterfaceState::new())
            .add_startup_system(events::register_events)
            .add_system(events::handle_events)
            .add_system(crosshair::crosshair)
            .add_system(health::health)
            .add_system(debug::debug)
            .add_system(menu::gamemenu::gamemenu)
            .add_system(menu::death::death);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InterfaceId(u32);

#[derive(Debug, Resource)]
pub struct InterfaceState {
    interfaces: HashSet<InterfaceId>,
}

impl InterfaceState {
    pub fn new() -> Self {
        Self {
            interfaces: HashSet::from([InterfaceId(0x01)]),
        }
    }

    pub fn open(&mut self, id: InterfaceId) {
        self.interfaces.insert(id);
    }

    pub fn close(&mut self, id: InterfaceId) {
        self.interfaces.remove(&id);
    }

    pub fn toggle(&mut self, id: InterfaceId) {
        if self.is_open(id) {
            self.close(id);
        } else {
            self.open(id);
        }
    }

    pub fn is_open(&self, id: InterfaceId) -> bool {
        self.interfaces.contains(&id)
    }
}
