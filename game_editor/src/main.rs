use std::path::PathBuf;

use backend::Backend;

use bevy_app::App;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::{Commands, Query};
use game_ui::events::Events;
use game_ui::render::style::Background;
use image::Rgba;
// use game_common::archive::loader::ModuleLoader;
// use game_common::archive::GameArchive;
// use game_common::world::world::WorldState;
// use game_core::CorePlugins;
// use game_input::InputPlugin;
use game_ui::render::layout::LayoutTree;
use game_ui::widgets::{Context, Widget};
use game_ui::UiPlugin;
use game_window::Window;
// use plugins::camera::CameraPlugin;
use tokio::runtime::Runtime;
use widgets::explorer::{Event, Explorer, ExplorerQueue};
use widgets::tool_bar::ToolBar;
use windows::SpawnWindowQueue;

mod backend;
mod widgets;
// mod picker;
// mod plugins;
mod state;
// mod ui;
mod windows;
// mod world;

fn main() {
    // let archive = GameArchive::new();

    // let loader = ModuleLoader::new(&archive);
    // loader.load("../mods/core").unwrap();

    let (backend, handle) = Backend::new();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(backend.run());
    });

    App::new()
        .insert_resource(handle)
        .insert_resource(ExplorerQueue::default())
        .add_plugin(UiPlugin)
        .add_startup_system(setup)
        .add_system(explorer_queue)
        .add_plugin(windows::WindowsPlugin)
        .run();
}

fn setup(mut commands: Commands, queue: Res<ExplorerQueue>, wqueue: Res<SpawnWindowQueue>) {
    let mut tree = LayoutTree::new();
    let mut events = Events::default();

    let mut ctx = Context {
        parent: None,
        tree: &mut tree,
        events: &mut events,
    };

    ToolBar {
        queue: wqueue.clone(),
    }
    .create(&mut ctx);

    let id = commands
        .spawn(Window {
            title: "main window".to_owned(),
        })
        .id();

    Explorer {
        window: id,
        queue: queue.clone(),
        path: PathBuf::from("./"),
    }
    .create(&mut ctx);

    commands.entity(id).insert(tree).insert(events);
}

fn explorer_queue(
    queue: Res<ExplorerQueue>,
    mut windows: Query<&mut LayoutTree>,
    mut commands: Commands,
) {
    let mut queue = queue.0.write();

    while let Some(event) = queue.pop_front() {
        match event {
            Event::Select {
                window,
                key,
                selected,
            } => {
                let mut window = windows.get_mut(window).unwrap();

                let elem = window.get_mut(key).unwrap();
                elem.style.background = if selected {
                    Background::Color(Rgba([70, 89, 88, 255]))
                } else {
                    Background::None
                };
            }
            Event::Open { entries } => {
                dbg!(entries);
            }
            Event::Cancel { window } => {
                commands.entity(window).despawn();
            }
        }
    }
}
