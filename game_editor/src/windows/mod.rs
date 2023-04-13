use bevy::prelude::{Camera, Camera3dBundle, Commands, EventReader};
use bevy::render::camera::RenderTarget;
use bevy::window::{Window, WindowRef};
use game_common::module::ModuleId;
use game_data::record::RecordId;

use crate::state::module::{EditorModule, Modules};
use crate::state::record::Records;

use self::error::ErrorWindowsPlugin;
use self::modules::ModuleWindowPlugin;
use self::records::RecordsWindowPlugin;

mod error;
mod modules;
mod records;

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    Modules,
    EditModule(EditorModule),
    CreateModule,
    ImportModule,
    Templates,
    Record(ModuleId, RecordId),
    CreateRecord,
    Error(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowPlugin;

impl bevy::prelude::Plugin for WindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<SpawnWindow>();
        app.add_plugin(ErrorWindowsPlugin);
        app.add_plugin(RecordsWindowPlugin);
        app.add_plugin(ModuleWindowPlugin);

        app.insert_resource(Records::new());
        app.insert_resource(Modules::new());

        app.add_system(spawn_window);
    }
}

fn spawn_window(mut events: EventReader<SpawnWindow>, mut commands: Commands) {
    for event in events.iter() {
        let mut cmds = commands.spawn(Window {
            title: "window".to_owned(),
            ..Default::default()
        });

        match event {
            SpawnWindow::Modules => {
                cmds.insert(modules::ModuleWindow);
            }
            SpawnWindow::EditModule(module) => {
                cmds.insert(modules::EditModuleWindow {
                    module: module.clone(),
                });
            }
            SpawnWindow::CreateModule => {
                cmds.insert(modules::CreateModuleWindow::new());
            }
            SpawnWindow::ImportModule => {
                cmds.insert(modules::LoadModuleWindow::default());
            }
            SpawnWindow::Templates => {
                cmds.insert(records::RecordsWindow::new());
            }
            SpawnWindow::Record(module, id) => {
                cmds.insert(records::RecordWindow {
                    module: *module,
                    id: *id,
                    record: None,
                });
            }
            SpawnWindow::CreateRecord => {
                cmds.insert(records::CreateRecordWindow::new());
            }
            SpawnWindow::Error(text) => {
                cmds.insert(error::ErrorWindow {
                    text: text.to_owned(),
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
