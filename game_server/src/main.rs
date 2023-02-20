use std::time::Duration;

use bevy::prelude::App;
use clap::Parser;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_core::CorePlugins;
use server::Server;
use snapshot::CommandQueue;
use state::State;
use tokio::time::{interval, MissedTickBehavior};

mod config;
mod conn;
mod plugins;
mod server;
mod snapshot;
mod state;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

#[tokio::main]
async fn main() {
    let state = State::new();

    let queue = state.queue.clone();

    tokio::task::spawn(async move {
        let server = Server::new(state).unwrap();
        server.await.unwrap();
    });

    let archive = GameArchive::new();

    let loader = ModuleLoader::new(&archive);
    loader.load("../mods/core").unwrap();

    let mut app = App::new();
    app.insert_resource(archive);
    app.add_plugin(CorePlugins);
    app.insert_resource(queue);

    let mut interval = interval(Duration::from_millis(50));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        app.update();
        interval.tick().await;
    }
}
