mod modules;
mod view;

use std::collections::VecDeque;
use std::sync::Arc;

use bevy_app::Plugin;
use bevy_ecs::prelude::{EventReader, EventWriter};
use bevy_ecs::system::{Commands, ResMut, Resource};
use game_ui::events::Events;
use game_ui::render::layout::LayoutTree;
use game_ui::widgets::Context;
use game_window::Window;
use parking_lot::RwLock;

pub struct WindowsPlugin;

impl Plugin for WindowsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_event::<SpawnWindow>();
        app.add_system(spawn_windows);
        app.add_system(spawn_window_queue);

        app.insert_resource(SpawnWindowQueue::default());

        app.add_system(view::reset_state_on_cursor_leave);
        app.add_system(view::zoom_scene);
        app.add_system(view::update_view_camera);
        app.add_system(view::update_camera_mode);
        app.add_system(view::update_origin);
    }
}

fn spawn_windows(mut commands: Commands, mut events: EventReader<SpawnWindow>) {
    for event in events.iter() {
        let mut tree = LayoutTree::new();
        let mut events = Events::default();

        let mut ctx = Context {
            parent: None,
            tree: &mut tree,
            events: &mut events,
        };

        match event {
            SpawnWindow::Modules => {
                let mut window = commands.spawn(Window {
                    title: "test".to_owned(),
                });

                modules::spawn_modules_window(&mut ctx);

                window.insert((tree, events));
            }
            SpawnWindow::CreateModule => todo!(),
            SpawnWindow::OpenModule => todo!(),
            SpawnWindow::View => {
                view::spawn_view_window(&mut commands);
            }
        }
    }
}

pub enum SpawnWindow {
    Modules,
    CreateModule,
    OpenModule,
    View,
}

#[derive(Resource, Default, Clone)]
pub struct SpawnWindowQueue(pub Arc<RwLock<VecDeque<SpawnWindow>>>);

fn spawn_window_queue(queue: ResMut<SpawnWindowQueue>, mut writer: EventWriter<SpawnWindow>) {
    let mut queue = queue.0.write();
    while let Some(event) = queue.pop_front() {
        writer.send(event);
    }
}
