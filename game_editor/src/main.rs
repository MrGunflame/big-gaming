use backend::{Backend, Handle, Response};

use bevy_app::App;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::{Commands, ResMut};
use game_scene::ScenePlugin;
use game_ui::events::Events;
use game_ui::reactive::Document;
// use game_common::archive::loader::ModuleLoader;
// use game_common::archive::GameArchive;
// use game_common::world::world::WorldState;
// use game_core::CorePlugins;
// use game_input::InputPlugin;
use game_ui::render::layout::LayoutTree;
use game_ui::render::style::{Background, BorderRadius, Bounds, Size, SizeVec2};
use game_ui::UiPlugin;
use game_window::Window;
use state::module::Modules;
use state::record::Records;
// use plugins::camera::CameraPlugin;
use tokio::runtime::Runtime;
use widgets::tool_bar::ToolBar;
use windows::SpawnWindowQueue;

use widgets::tool_bar::*;
use windows::modules::CreateModules;

use crate::windows::SpawnWindow;

mod backend;
mod widgets;
// mod picker;
// mod plugins;
mod state;
// mod ui;
mod windows;
// mod world;

fn main() {
    pretty_env_logger::init();

    // let archive = GameArchive::new();

    // let loader = ModuleLoader::new(&archive);
    // loader.load("../mods/core").unwrap();

    let (backend, handle) = Backend::new();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(backend.run());
    });

    App::new()
        .insert_resource(Modules::new())
        .insert_resource(Records::new())
        .insert_resource(handle)
        .add_plugin(ScenePlugin)
        .add_plugin(UiPlugin)
        .add_startup_system(setup)
        .add_plugin(windows::WindowsPlugin)
        .add_system(load_from_backend)
        .run();
}

fn setup(
    mut commands: Commands,
    queue: Res<SpawnWindowQueue>,
    rt: Res<game_ui::reactive::Runtime>,
) {
    let mut tree = LayoutTree::new();
    let mut events = Events::default();

    // let mut ctx = Context {
    //     parent: None,
    //     tree: &mut tree,
    //     events: &mut events,
    // };

    // ToolBar {
    //     queue: wqueue.clone(),
    // }
    // .create(&mut ctx);

    let id = commands
        .spawn(Window {
            title: "main window".to_owned(),
        })
        .id();

    // Explorer {
    //     window: id,
    //     queue: queue.clone(),
    //     path: PathBuf::from("./"),
    // }
    // .create(&mut ctx);

    let document = Document::new(rt.clone());

    let buttons = vec![
        ActionButton {
            label: "Modules".to_owned(),
            on_click: {
                let queue = queue.clone();
                Box::new(move |_| {
                    let mut queue = queue.0.write();
                    queue.push_back(SpawnWindow::Modules);
                })
            },
        },
        ActionButton {
            label: "Records".to_owned(),
            on_click: {
                let queue = queue.clone();

                Box::new(move |_| {
                    let mut queue = queue.0.write();
                    queue.push_back(SpawnWindow::Records);
                })
            },
        },
        ActionButton {
            label: "View".to_owned(),
            on_click: {
                let queue = queue.clone();
                Box::new(move |_| {
                    let mut queue = queue.0.write();
                    queue.push_back(SpawnWindow::View);
                })
            },
        },
    ];

    let cx = document.root_scope();
    game_ui::view! {
        cx,
        <ToolBar buttons={buttons}>
        </ToolBar>
    };

    let style = Style {
        background: Background::RED,
        bounds: Bounds {
            min: SizeVec2::splat(Size::Pixels(64.0)),
            max: SizeVec2::splat(Size::Pixels(64.0)),
        },
        border_radius: BorderRadius::splat(Size::Pixels(16.0)),
        ..Default::default()
    };

    game_ui::view! {cx,
        <Container style={style}>
        </Container>
    };

    use game_ui::render::style::Style;
    use game_ui::widgets::*;

    commands
        .entity(id)
        .insert(tree)
        .insert(events)
        .insert(document);
}

fn load_from_backend(
    handle: Res<Handle>,
    mut modules: ResMut<Modules>,
    mut records: ResMut<Records>,
    mut queue: Res<SpawnWindowQueue>,
    create_modules: Res<CreateModules>,
) {
    while let Some(resp) = handle.recv() {
        match resp {
            Response::LoadModule(res) => match res {
                Ok((module, recs)) => {
                    for (_, rec) in recs.iter() {
                        records.insert(module.module.id, rec.clone());
                    }

                    modules.insert(module.clone());

                    let inner = create_modules.0.lock();
                    if let Some(sig) = &*inner {
                        sig.update(|v| v.push(module))
                    }
                }
                Err(err) => {
                    tracing::error!("failed to load module: {}", err);

                    let msg = format!("failed to load module: {}", err);

                    let mut queue = queue.0.write();
                    queue.push_back(SpawnWindow::Error(msg));
                }
            },
            Response::WriteModule(res) => match res {
                Ok(()) => {}
                Err(err) => {
                    let mut queue = queue.0.write();
                    queue.push_back(SpawnWindow::Error(format!(
                        "failed to save modules: {}",
                        err
                    )));
                }
            },
        }
    }
}
