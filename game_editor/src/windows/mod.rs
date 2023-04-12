use std::collections::HashMap;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use std::sync::Arc;

use bevy::prelude::{Camera, Camera3dBundle, Commands, EventReader, ResMut, Resource};
use bevy::render::camera::RenderTarget;
use bevy::window::{Window, WindowRef};
use game_common::module::{Module, ModuleId};
use game_data::record::RecordId;
use game_data::DataBuffer;
use parking_lot::RwLock;

use crate::state::module::Records;

use self::modules::ModuleWindowPlugin;
use self::records::RecordsWindowPlugin;

mod modules;
mod records;

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    Modules,
    CreateModule,
    Templates,
    Record(Records, RecordId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowPlugin;

impl bevy::prelude::Plugin for WindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<SpawnWindow>();
        app.add_plugin(RecordsWindowPlugin);
        app.add_plugin(ModuleWindowPlugin);

        app.insert_resource(Forms::default());
        app.insert_resource(Modules::new());

        app.add_system(spawn_window);
    }
}

fn spawn_window(
    mut events: EventReader<SpawnWindow>,
    mut commands: Commands,
    mut forms: ResMut<Forms>,
) {
    for event in events.iter() {
        let mut cmds = commands.spawn(Window {
            title: "window".to_owned(),
            ..Default::default()
        });

        match event {
            SpawnWindow::Modules => {
                cmds.insert(modules::ModuleWindow);
            }
            SpawnWindow::CreateModule => {
                cmds.insert(modules::CreateModuleWindow::new());
            }
            SpawnWindow::Templates => {
                let form = forms.modules.entry(ModuleId::default()).or_default();

                cmds.insert(records::RecordsWindow::new(
                    ModuleId::default(),
                    Records::default(),
                ));
            }
            SpawnWindow::Record(records, id) => {
                cmds.insert(records::RecordWindow {
                    records: records.clone(),
                    id: *id,
                });
            }
        }

        let id = cmds.id();

        commands.spawn(Camera3dBundle {
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(id)),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}

#[derive(Clone, Debug, Default, Resource)]
pub struct Forms {
    pub modules: HashMap<ModuleId, Arc<RwLock<DataBuffer>>>,
}

#[derive(Clone, Debug, Default, Resource)]
pub struct Modules {
    pub modules: HashMap<ModuleId, ModuleData>,
}

impl Modules {
    pub fn new() -> Self {
        let mut modules = HashMap::new();
        modules.insert(
            ModuleId::default(),
            ModuleData {
                module: Module::core(),
                capabilities: Capabilities::READ,
            },
        );

        Self { modules }
    }
}

#[derive(Clone, Debug)]
pub struct ModuleData {
    pub module: Module,
    pub capabilities: Capabilities,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities(u8);

impl Capabilities {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(1 << 1);

    #[inline]
    pub fn read(self) -> bool {
        (self & Self::READ) != Self::NONE
    }

    #[inline]
    pub fn write(self) -> bool {
        (self & Self::WRITE) != Self::NONE
    }
}

impl BitAnd for Capabilities {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Capabilities {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for Capabilities {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Capabilities {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
