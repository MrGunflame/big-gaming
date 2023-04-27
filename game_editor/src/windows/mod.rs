use bevy::prelude::{Camera, Camera3dBundle, Commands, EventReader};
use bevy::render::camera::RenderTarget;
use bevy::window::{Window, WindowRef};
use game_common::module::ModuleId;
use game_common::record::RecordId;
use game_data::record::RecordKind;
use game_data::uri::Uri;

use crate::state::module::{EditorModule, Modules};
use crate::state::record::Records;

use self::error::ErrorWindowsPlugin;
use self::files::FilesWindowPlugin;
use self::modules::ModuleWindowPlugin;
use self::records::RecordsWindowPlugin;

mod error;
mod files;
mod modules;
mod records;
mod view;

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    Modules,
    EditModule(EditorModule),
    CreateModule,
    ImportModule,
    Templates,
    Record(ModuleId, RecordId),
    CreateRecord(RecordKind),
    Error(String),
    View(Uri),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowPlugin;

impl bevy::prelude::Plugin for WindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<SpawnWindow>();
        app.add_plugin(ErrorWindowsPlugin);
        app.add_plugin(RecordsWindowPlugin);
        app.add_plugin(ModuleWindowPlugin);
        app.add_plugin(FilesWindowPlugin);

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
                cmds.insert(files::OpenFilesWindow::new());
            }
            SpawnWindow::Templates => {
                cmds.insert(records::RecordsWindow::new());
            }
            SpawnWindow::Record(module, id) => {
                cmds.insert(records::RecordWindow {
                    module: *module,
                    id: *id,
                    record: None,
                    add_action: 0,
                    add_comp: 0,
                });
            }
            SpawnWindow::CreateRecord(kind) => {
                cmds.insert(records::CreateRecordWindow::new(*kind));
            }
            SpawnWindow::Error(text) => {
                cmds.insert(error::ErrorWindow {
                    text: text.to_owned(),
                });
            }
            SpawnWindow::View(uri) => {
                cmds.insert(view::ViewWindow::new(uri.clone()));
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
