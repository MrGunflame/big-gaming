use std::time::Duration;

use bevy::prelude::App;
use clap::Parser;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_core::CorePlugins;
use plugins::ServerPlugins;
use server::Server;
use state::State;
use tokio::time::{interval, MissedTickBehavior};
use world::WorldPlugin;

mod config;
mod conn;
mod entity;
mod net;
pub mod plugins;
mod server;
mod snapshot;
mod state;
mod world;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

#[tokio::main]
async fn main() {
    game_core::logger::init();

    let state = State::new();

    let queue = state.queue.clone();
    let conns = state.conns.clone();

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

    let archive = GameArchive::new();

    let loader = ModuleLoader::new(&archive);
    loader.load("../mods/core").unwrap();

    let mut app = App::new();
    app.insert_resource(archive);
    app.add_plugin(CorePlugins);

    app.insert_resource(queue);
    app.insert_resource(conns);
    app.add_plugin(ServerPlugins);

    app.add_plugin(WorldPlugin);

    app.insert_resource(game_physics::Pipeline::new());

    let timestep = Duration::from_secs(1) / 20;

    let mut interval = interval(timestep.into());
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        app.update();
        interval.tick().await;
    }
}
