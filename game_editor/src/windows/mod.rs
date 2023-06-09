mod create_module;
mod create_record;
mod error;
pub mod modules;
mod open_module;
mod records;
mod view;

use std::collections::VecDeque;
use std::sync::Arc;

use bevy_app::Plugin;
use bevy_ecs::prelude::{EventReader, EventWriter, Res};
use bevy_ecs::system::{Commands, ResMut, Resource};
use game_data::record::RecordKind;
use game_ui::events::Events;
use game_ui::reactive::{Document, Runtime};
use game_ui::render::layout::LayoutTree;
use game_ui::view;
use game_window::Window;
use parking_lot::RwLock;

use crate::backend::Handle;

use self::create_module::*;
use self::create_record::*;
use self::error::*;
use self::modules::*;
use self::open_module::*;
use self::records::*;

pub struct WindowsPlugin;

impl Plugin for WindowsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_event::<SpawnWindow>();
        app.add_system(spawn_windows);
        app.add_system(spawn_window_queue);

        app.insert_resource(SpawnWindowQueue::default());

        app.insert_resource(CreateModules::default());

        app.add_system(view::reset_state_on_cursor_leave);
        app.add_system(view::zoom_scene);
        app.add_system(view::update_view_camera);
        app.add_system(view::update_camera_mode);
        app.add_system(view::update_origin);
    }
}

fn spawn_windows(
    mut commands: Commands,
    mut events: EventReader<SpawnWindow>,
    queue: ResMut<SpawnWindowQueue>,
    handle: Res<Handle>,
    records: Res<crate::state::record::Records>,
    create_modules: Res<CreateModules>,
    rt: Res<Runtime>,
    modules: Res<crate::state::module::Modules>,
) {
    for event in events.iter() {
        let window = Window {
            title: "test".to_owned(),
        };

        let tree = LayoutTree::new();
        let events = Events::default();
        let document = Document::new(rt.clone());

        // let mut ctx = Context {
        //     parent: None,
        //     tree: &mut tree,
        //     events: &mut events,
        // };

        // match event {
        //     SpawnWindow::Modules => {
        //         let mut window = commands.spawn(Window {
        //             title: "test".to_owned(),
        //         });

        //         modules::spawn_modules_window(&mut ctx);

        //         window.insert((tree, events));
        //     }
        //     SpawnWindow::CreateModule => todo!(),
        //     SpawnWindow::OpenModule => todo!(),
        //     SpawnWindow::View => {
        //         view::spawn_view_window(&mut commands);
        //     }
        // }

        if let SpawnWindow::View = event {
            view::spawn_view_window(&mut commands);
            continue;
        }

        let mut cmds = commands.spawn(window);
        let id = cmds.id();

        let cx = document.root_scope();
        match event {
            SpawnWindow::Modules => {
                view! {
                    cx,
                    <Modules queue={queue.clone()} modules={modules.clone()} create_modules={create_modules.clone()}>
                    </Modules>
                };
            }
            SpawnWindow::OpenModule => {
                view! {
                    cx,
                    <OpenModule window={id} handle={handle.clone()}>
                    </OpenModule>
                };
            }
            SpawnWindow::CreateModule => {
                view! {
                    cx,
                    <CreateModule modules={modules.clone()}>
                    </CreateModule>
                };
            }
            SpawnWindow::Error(msg) => {
                view! {
                    cx,
                    <Error message={msg}>
                    </Error>
                };
            }
            SpawnWindow::Records => {
                view! {
                    cx,
                    <Records queue={queue.clone()}>
                    </Records>
                };
            }
            SpawnWindow::CreateRecord(kind) => {
                view! {
                    cx,
                    <CreateRecord kind={*kind} records={records.clone()} modules={modules.clone()}>
                    </CreateRecord>
                };
            }
            _ => todo!(),
        }

        cmds.insert((tree, events, document));
    }
}

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    Modules,
    CreateModule,
    OpenModule,
    Records,
    View,
    Error(String),
    CreateRecord(RecordKind),
}

#[derive(Resource, Default, Clone)]
pub struct SpawnWindowQueue(pub Arc<RwLock<VecDeque<SpawnWindow>>>);

fn spawn_window_queue(queue: ResMut<SpawnWindowQueue>, mut writer: EventWriter<SpawnWindow>) {
    let mut queue = queue.0.write();
    while let Some(event) = queue.pop_front() {
        writer.send(event);
    }
}
