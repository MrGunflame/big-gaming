use bevy::prelude::{Camera, Camera3dBundle, Commands, EventReader};
use bevy::render::camera::RenderTarget;
use bevy::window::{Window, WindowRef};
use game_common::module::ModuleId;
use game_data::record::RecordId;

use crate::state::module::{Modules, Records};

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

        app.insert_resource(Records::default());
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
            SpawnWindow::CreateModule => {
                cmds.insert(modules::CreateModuleWindow::new());
            }
            SpawnWindow::Templates => {
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
