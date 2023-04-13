use backend::Backend;
use bevy::a11y::AccessibilityPlugin;

use bevy::core_pipeline::CorePipelinePlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::{
    shape, App, Assets, Color, Commands, GilrsPlugin, ImagePlugin, Mesh, PbrBundle, PointLight,
    PointLightBundle, ResMut, StandardMaterial, Transform,
};
use bevy::render::RenderPlugin;
use bevy::sprite::SpritePlugin;
use bevy::text::TextPlugin;
use bevy::window::WindowPlugin;
use bevy::winit::WinitPlugin;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_common::world::world::WorldState;
use game_core::CorePlugins;
use game_input::InputPlugin;
use game_ui::{InterfaceState, UiPlugin};
use plugins::camera::CameraPlugin;
use tokio::runtime::Runtime;
use world::EntityOptions;

mod backend;
mod picker;
mod plugins;
mod state;
mod ui;
mod windows;
mod world;

fn main() {
    let archive = GameArchive::new();

    let loader = ModuleLoader::new(&archive);
    loader.load("../mods/core").unwrap();

    let (backend, handle) = Backend::new();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(backend.run());
    });

    App::new()
        .insert_resource(handle)
        .insert_resource(archive)
        .insert_resource(WorldState::new())
        .add_plugin(CorePlugins)
        .add_plugin(AccessibilityPlugin)
        .add_plugin(WindowPlugin::default())
        .add_plugin(RenderPlugin::default())
        .add_plugin(ImagePlugin::default())
        .add_plugin(CorePipelinePlugin::default())
        .add_plugin(PbrPlugin::default())
        .add_plugin(SpritePlugin)
        .add_plugin(TextPlugin)
        .add_plugin(bevy::ui::UiPlugin)
        .add_plugin(GilrsPlugin)
        .add_plugin(WinitPlugin)
        .add_plugin(CameraPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        // .add_plugin(UiPlugin)
        .add_startup_system(setup)
        .add_system(world::axes::render_axes)
        .add_plugin(windows::WindowPlugin)
        .add_system(ui::main_bar::render_main_bar)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // mut interface: ResMut<InterfaceState>,
) {
    // interface.push(ui::SceneHierarchy::default());

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::from_size(5.0))),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });

    // cube
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..Default::default()
        })
        .insert(EntityOptions {
            selected: false,
            hidden: false,
        });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });

    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(
    //         Axis {
    //             direction: Vec3::X,
    //             length: 1.0,
    //         }
    //         .into(),
    //     ),
    //     material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //     transform: Transform::from_xyz(0.0, 2.5, 0.0),
    //     ..Default::default()
    // });
}
