use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::{Camera, Camera3dBundle, Commands, EventReader, ResMut, Resource};
use bevy::render::camera::RenderTarget;
use bevy::window::{Window, WindowRef};
use game_common::module::ModuleId;
use game_data::DataBuffer;
use parking_lot::RwLock;

use self::modules::ModuleWindowPlugin;
use self::templates::TemplatesPlugin;

mod modules;
mod templates;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpawnWindow {
    Modules,
    Templates,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowPlugin;

impl bevy::prelude::Plugin for WindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<SpawnWindow>();
        app.add_plugin(TemplatesPlugin);
        app.add_plugin(ModuleWindowPlugin);

        app.insert_resource(Forms::default());

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
            SpawnWindow::Templates => {
                let form = forms.modules.entry(ModuleId::default()).or_default();

                cmds.insert(templates::TemplatesWindow {
                    module: ModuleId::default(),
                    data: form.clone(),
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
