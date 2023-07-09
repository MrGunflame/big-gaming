#![deny(unsafe_op_in_unsafe_fn)]

mod config;
mod entities;
mod net;
mod plugins;
mod state;
mod utils;
mod window;

use bevy_app::App;
use bevy_ecs::system::Commands;
use clap::Parser;
use config::Config;
use entities::LoadEntityPlugin;
use game_core::logger::{self};
use game_core::CorePlugins;
use game_render::light::{DirectionalLight, DirectionalLightBundle};
use game_render::RenderPlugin;
use game_scene::ScenePlugin;
use net::NetPlugin;
use plugins::actions::ActionsPlugin;
use plugins::{CameraPlugin, MovementPlugin};
use state::InternalGameState;
use window::PrimaryWindow;

use crate::net::ServerConnection;

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connect: Option<String>,
}

fn main() {
    logger::init();

    let args = Args::parse();

    let mut config_path = std::env::current_dir().unwrap();
    config_path.push("config.toml");
    let config = match Config::from_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("failed to load config file from {:?}: {}", config_path, err);
            return;
        }
    };

    let mut app = App::new();

    app.insert_resource(config);

    app.add_plugin(RenderPlugin);

    app.init_resource::<InternalGameState>();
    app.add_system(state::update_game_state);

    // Window setup
    app.init_resource::<PrimaryWindow>();
    app.add_system(window::destroy_primary_window);

    app.add_plugin(CorePlugins);
    app.add_plugin(NetPlugin::default());
    app.add_plugin(ActionsPlugin);
    app.add_plugin(LoadEntityPlugin);
    app.add_plugin(CameraPlugin);
    app.add_plugin(ScenePlugin);
    app.add_plugin(MovementPlugin);

    app.add_startup_system(spawn_light);

    game_core::modules::load_modules(&mut app);

    if let Some(addr) = args.connect {
        tracing::info!("Connecting to {}", addr);

        app.world.resource_mut::<ServerConnection>().connect(addr);
    }

    app.run();
}

fn spawn_light(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        light: DirectionalLight {
            color: [1.0, 1.0, 1.0],
            illuminance: 1.0,
        },
        transform: Default::default(),
    });
}
