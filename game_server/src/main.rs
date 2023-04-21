use bevy::prelude::App;
use clap::Parser;

use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_core::CorePlugins;
use game_server::config::Config;
use game_server::plugins::ServerPlugins;
use game_server::state::State;
use game_server::world::WorldPlugin;
use tokio::runtime::Runtime;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

fn main() {
    game_core::logger::init();

    let config = Config::from_file("./config.toml").unwrap();

    let state = State::new(config);

    let queue = state.queue.clone();
    let conns = state.conns.clone();

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

    let rt = Runtime::new().unwrap();
    rt.block_on(game_server::run(app, state));
}
