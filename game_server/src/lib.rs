pub mod config;
pub mod conn;
pub mod entity;
pub mod net;
pub mod plugins;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod world;

use std::time::Duration;

use bevy_app::App;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_core::CorePlugins;
use plugins::ServerPlugins;
use tokio::time::{interval, MissedTickBehavior};
use world::WorldPlugin;

use crate::config::Config;
use crate::server::Server;
use crate::state::State;

pub fn prepare(app: &mut App) {
    app.add_plugin(CorePlugins);

    app.add_plugin(WorldPlugin);

    app.insert_resource(game_physics::Pipeline::new());
}

pub async fn run(mut app: App, config: Config) {
    let state = State::new(config);

    let queue = state.queue.clone();
    let conns = state.conns.clone();

    let archive = GameArchive::new();
    let loader = ModuleLoader::new(&archive);
    loader.load("../mods/core").unwrap();
    app.insert_resource(archive);

    app.insert_resource(queue);
    app.insert_resource(conns);

    app.add_plugin(ServerPlugins);
    app.insert_resource(state.clone());

    let timestep = Duration::from_secs(1) / state.config.timestep;

    tokio::task::spawn(async move {
        let server = match Server::new(state) {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("failed to run server: {}", err);
                return;
            }
        };

        if let Err(err) = server.await {
            tracing::error!("failed to run server: {}", err);
        }
    });

    let mut interval = interval(timestep.into());
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        app.update();
        interval.tick().await;

        *app.world.resource::<State>().control_frame.lock() += 1;
    }
}
